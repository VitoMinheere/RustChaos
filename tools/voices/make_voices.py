"""Voices for the unreleased missions' lines that were never recorded, made with Chatterbox.

    make_voices.py EXPORT OUT [--only VOICE,...] [--missions M,...] [--briefings] [--annoyed] [--chatter]

EXPORT is what `cargo run --example voice_lines -- EXPORT` wrote (lines.tsv, and the
recorded lines under clips/). OUT gets talk2/<mission>.ucm<speech><letter>.wav, named as
the game's own, for the port to play where the install has none, and made.tsv listing
what each file is. Lines already made are skipped, so a run can be stopped and picked up.

A line some other mission has a recording of, word for word and in the same voice, gets
that recording instead. voices.json says, per voice, which recording to copy the voice
from and how much feeling to put in; overrides.json gives a line other words to say (by
"<mission>.ucm<speech><letter>"), or null for none.

--briefings makes the unreleased missions' spoken briefings instead, which the game never
had: briefings.json gives each file (talk2/<mission>.brief.wav, as the menu looks for it)
its voice and words.

--annoyed makes the frightened citizens' lines, which were never recorded: each of the
install's text/annoyed.txt (talk2/misc/fcit-annoyed01.wav for a woman, mcit-annoyed01.wav for
a man, and so on), in the voices of the citizens' own recordings (voices.json's "fcit" and
"mcit", whose "install:" samples are read from the install: --game, else UC_GAME_DIR).

--chatter makes what the crowds say when spoken to, which was shown as text only: every line
of the chatter files the missions read (CHATTER), in both citizens' voices, as
talk2/misc/fcit-chat<key>.wav and mcit-chat<key>.wav, where <key> is chatter_key() of the
line (the port's `uc_game::city::chatter_key`), so a line in several files is made once. The
lines the citizens recorded themselves (RECORDED) are left out for the voice that has them.
"""
import argparse, csv, json, os, re, shutil, time

import torch
import torchaudio

here = os.path.dirname(os.path.abspath(__file__))

# The missions the game never released (the viewer's UNRELEASED and DRAFTS).
UNRELEASED = {"FreeCD1", "jung3", "train11", "snow1", "Takeover1", "subdock1", "bball1", "Gang order3", "fight3",
              "BAALROG1", "gang1", "gang3", "Getaway1", "Transmit1", "army1", "upperclass1", "estate1", "Crimescene",
              "police5", "Finale1b", "carbomb2"}
# The game's voice lines are 22 kHz mono.
RATE = 22050


def words(t):
    return re.sub(r"[^a-z0-9 ]", "", t.lower().replace("\x92", "'")).split()


def spoken(t):
    """The script's text as something to say: shouted capitals as words, not letters."""
    t = t.replace("\x92", "'")
    t = re.sub(r"\b([A-Z]{2,}[A-Z']*)\b", lambda m: m.group(1) if m.group(1) in ("TV", "OK", "UC") else m.group(1).lower(), t)
    # A `|` starts a new line on screen; " : " is a heading's.
    t = t.replace("|", ". ").replace(" : ", ". ").replace(" - ", ", ").replace("....", "...")
    t = re.sub(r"\b(\d+)(st|nd|rd|th)\b", lambda m: ORDINALS.get(m.group(1), m.group(0)), t)
    t = re.sub(r"(?<![.\d])\b(\d{1,2})\b(?!\.\d)", lambda m: NUMBERS[int(m.group(1))] if int(m.group(1)) < len(NUMBERS) else m.group(1), t)
    return re.sub(r"\s+", " ", t).strip()


ORDINALS = {"1": "first", "2": "second", "3": "third", "4": "fourth", "5": "fifth", "12": "twelfth", "24": "twenty-fourth"}
NUMBERS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
           "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen", "twenty"]


# The chatter files the missions read, the port's fallbacks for missing ones included
# (crates/uc-game/src/city.rs, `chatter_file`). Not park23.txt or silly.txt: nothing names them.
CHATTER = ["citsez.txt", "police11.txt", "police21.txt", "police31.txt", "police41.txt", "mission21.txt", "E31.txt",
           "Gangorder11.txt", "Westcrime11.txt", "bball24.txt", "estate21.txt", "factory12.txt", "skymiss21.txt",
           "wstores11.txt"]
# citsez.txt's lines the citizens recorded (talk2/misc/fcit-sez*, mcit-sez*), by voice.
RECORDED = {"fcit": [1, 6, 13, 20], "mcit": [1, 3, 6, 10]}


def line_key(t):
    return "".join(c.lower() for c in t.replace("\x92", "'") if c.isascii() and c.isalnum())


def chatter_key(t):
    """FNV-1a of the line's letters and digits, lower case, as 8 hex digits."""
    h = 0x811C9DC5
    for b in line_key(t).encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return f"{h:08x}"


def numbered(path):
    text = open(path, encoding="latin-1").read()
    return [(int(m.group(1)), m.group(2).strip()) for m in re.finditer(r"^\s*(\d+)\.\s*(.+)$", text, flags=re.M) if m.group(2).strip()]


def rms(x):
    return x.pow(2).mean().sqrt().item()


def main():
    a = argparse.ArgumentParser()
    a.add_argument("export")
    a.add_argument("out")
    a.add_argument("--only", help="voices to make, comma separated")
    a.add_argument("--missions", help="missions to make, comma separated")
    a.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    a.add_argument("--briefings", action="store_true", help="the unreleased missions' briefings")
    a.add_argument("--annoyed", action="store_true", help="the frightened citizens' lines (annoyed.txt)")
    a.add_argument("--chatter", action="store_true", help="what the crowds say (the chatter files)")
    a.add_argument("--game", default=os.environ.get("UC_GAME_DIR"), help="the install, for --annoyed and --chatter")
    args = a.parse_args()
    voices = json.load(open(os.path.join(here, "voices.json")))
    talk2 = os.path.join(args.out, "talk2")
    os.makedirs(talk2, exist_ok=True)
    log = open(os.path.join(args.out, "made.tsv"), "a", encoding="utf-8")
    model = None
    loudness = {}

    def make(name, voice, text):
        nonlocal model
        if model is None:
            from chatterbox.tts import ChatterboxTTS
            model = ChatterboxTTS.from_pretrained(device=args.device)
        v = voices[voice]
        sample = v["sample"]
        sample = os.path.join(args.game, sample[len("install:"):]) if sample.startswith("install:") else os.path.join(args.export, sample)
        if voice not in loudness:
            s, _ = torchaudio.load(sample)
            loudness[voice] = rms(s)
        t = time.time()
        wav = model.generate(text, audio_prompt_path=sample, exaggeration=v["exaggeration"], cfg_weight=v["cfg_weight"])
        wav = torchaudio.functional.resample(wav.mean(0, keepdim=True), model.sr, RATE)
        # As loud as the recording it copies.
        wav = (wav * loudness[voice] / max(rms(wav), 1e-6)).clamp(-1, 1)
        os.makedirs(os.path.dirname(os.path.join(talk2, name + ".wav")), exist_ok=True)
        torchaudio.save(os.path.join(talk2, name + ".wav"), wav, RATE, encoding="PCM_S", bits_per_sample=16)
        log.write(f"{name}\t{voice}\tmade\t{time.time() - t:.1f}s\t{text}\n")
        log.flush()
        print(f"{name} ({voice}): {text}", flush=True)

    if args.annoyed:
        if not args.game:
            raise SystemExit("--annoyed needs the install: --game DIR or UC_GAME_DIR")
        text = open(os.path.join(args.game, "text", "annoyed.txt"), encoding="latin-1").read()
        lines = [(int(m.group(1)), m.group(2).strip()) for m in re.finditer(r"^\s*(\d+)\.\s*(.+)$", text, flags=re.M)]
        for voice in ("fcit", "mcit"):
            if args.only and voice not in args.only.split(","):
                continue
            for n, line in lines:
                name = f"misc/{voice}-annoyed{n:02}"
                if not os.path.exists(os.path.join(talk2, name + ".wav")):
                    make(name, voice, spoken(line))
        return

    if args.chatter:
        if not args.game:
            raise SystemExit("--chatter needs the install: --game DIR or UC_GAME_DIR")
        recorded = {v: {line_key(t) for n, t in numbered(os.path.join(args.game, "text", "citsez.txt")) if n in ns}
                    for v, ns in RECORDED.items()}
        seen = set()
        for file in CHATTER:
            for _, line in numbered(os.path.join(args.game, "text", file)):
                key = chatter_key(line)
                if key in seen:
                    continue
                seen.add(key)
                for voice in ("fcit", "mcit"):
                    if (args.only and voice not in args.only.split(",")) or line_key(line) in recorded[voice]:
                        continue
                    name = f"misc/{voice}-chat{key}"
                    if not os.path.exists(os.path.join(talk2, name + ".wav")):
                        make(name, voice, spoken(line))
        return

    if args.briefings:
        briefings = json.load(open(os.path.join(here, "briefings.json"), encoding="utf-8"))
        for file, b in briefings.items():
            name = file.removesuffix(".wav")
            mission = name.removesuffix(".brief")
            if (args.only and b["voice"] not in args.only.split(",")) or (args.missions and mission not in args.missions.split(",")):
                continue
            if not os.path.exists(os.path.join(talk2, file)):
                make(name, b["voice"], spoken(b["text"]))
        return

    overrides = json.load(open(os.path.join(here, "overrides.json"), encoding="utf-8"))
    only = set(args.only.split(",")) if args.only else set(voices)
    missions = set(args.missions.split(",")) if args.missions else UNRELEASED
    rows = list(csv.DictReader(open(os.path.join(args.export, "lines.tsv"), encoding="utf-8", errors="replace"),
                               delimiter="\t", quoting=csv.QUOTE_NONE))
    recorded = {}
    for r in rows:
        if r["recorded"] == "1":
            recorded.setdefault((" ".join(words(r["text"])), r["voice"]), r)
    for r in rows:
        if r["mission"] not in missions or r["recorded"] == "1" or r["voice"] not in only:
            continue
        name = f'{r["mission"]}.ucm{r["speech"]}{r["line"]}'
        path = os.path.join(talk2, name + ".wav")
        if os.path.exists(path):
            continue
        text = overrides.get(name, spoken(r["text"]))
        if not text or not re.search(r"[A-Za-z]", text):
            continue
        same = recorded.get((" ".join(words(r["text"])), r["voice"]))
        if same:
            src = os.path.join(args.export, "clips", same["voice"], f'{same["mission"]}.ucm{same["speech"]}{same["line"]}.wav')
            shutil.copy(src, path)
            log.write(f"{name}\t{r['voice']}\trecorded\t{same['mission']}.ucm{same['speech']}{same['line']}\t{r['text']}\n")
            log.flush()
            continue
        make(name, r["voice"], text)


if __name__ == "__main__":
    main()

"""Voices for the unreleased missions' lines that were never recorded, made with Chatterbox.

    make_voices.py EXPORT OUT [--only VOICE,...] [--missions M,...] [--briefings]

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
"""
import argparse, csv, json, os, re, shutil, time

import torch
import torchaudio

here = os.path.dirname(os.path.abspath(__file__))

# The missions the game never released (the viewer's UNRELEASED and DRAFTS).
UNRELEASED = {"FreeCD1", "jung3", "train11", "snow1", "Takeover1", "subdock1", "bball1", "Gang order3", "fight3",
              "BAALROG1", "gang1", "gang3", "Getaway1", "Transmit1", "estate1", "Crimescene", "police5", "Finale1b"}
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
        sample = os.path.join(args.export, v["sample"])
        if voice not in loudness:
            s, _ = torchaudio.load(sample)
            loudness[voice] = rms(s)
        t = time.time()
        wav = model.generate(text, audio_prompt_path=sample, exaggeration=v["exaggeration"], cfg_weight=v["cfg_weight"])
        wav = torchaudio.functional.resample(wav.mean(0, keepdim=True), model.sr, RATE)
        # As loud as the recording it copies.
        wav = (wav * loudness[voice] / max(rms(wav), 1e-6)).clamp(-1, 1)
        torchaudio.save(os.path.join(talk2, name + ".wav"), wav, RATE, encoding="PCM_S", bits_per_sample=16)
        log.write(f"{name}\t{voice}\tmade\t{time.time() - t:.1f}s\t{text}\n")
        log.flush()
        print(f"{name} ({voice}): {text}", flush=True)

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

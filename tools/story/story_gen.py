"""Builds docs/STORY.md and docs/scripts/*.md from the mission files.

Usage: python story_gen.py <ucall dir> <goals.json> <game dir> <notes.json> <out.md> [--draft <dir>]

Writes the overview to <out.md> and one file per mission to scripts/ beside it.

The mission files are exported to JSON by the `uc` tool (one per mission in <ucall dir>).
<notes.json> holds what can't be read from the files: names for the people, synopses,
names for the unreleased missions. With --draft, also writes one file per mission (no
notes needed) for reading.
"""
import json, os, re, sys
from collections import defaultdict, Counter

B = chr(92)

ET = {0: 'nobody', 1: 'civilian', 2: 'civilian with a balloon', 3: 'woman', 4: 'large woman', 5: 'workman',
      6: 'Wildcat (rasta)', 7: 'Wildcat (red)', 8: 'Wildcat (grey)', 9: 'Wildcat (rasta, pistol)',
      10: 'Wildcat (red, shotgun)', 11: 'Wildcat (grey, AK-47)', 12: 'cop', 13: 'cop (pistol)',
      14: 'cop (shotgun)', 15: 'cop (AK-47)', 16: 'hostage', 17: 'workman (grenades)', 18: 'tramp',
      19: 'Man In Black', 20: 'Man In Black', 21: 'Man In Black', 22: 'Darci', 23: 'Roper'}
AI = {0: 'stands about', 1: 'civilian', 2: 'guard', 3: 'assassin', 4: 'boss', 5: 'cop', 6: 'gang member',
      7: 'doorman', 8: 'bodyguard', 9: 'driver', 10: 'bomb disposal', 11: 'biker', 12: 'training dummy',
      13: 'bully', 14: 'cop driver', 15: 'a body (dies at once)', 16: 'runs from the player', 17: 'kills a colour',
      18: 'Man In Black', 19: 'Bane', 20: 'lies injured', 21: 'sniper/assassin'}
PLAYER = {1: 'Darci', 2: 'Roper', 3: 'a cop', 4: 'Mako'}
IT = {1: 'key', 2: 'pistol', 3: 'health pack', 4: 'shotgun', 5: 'knife', 6: 'AK-47', 7: 'mine', 8: 'baseball bat',
      9: 'barrels', 10: 'pistol ammo', 11: 'shotgun ammo', 12: 'AK-47 ammo', 13: 'keycard', 14: 'file',
      15: 'floppy disk', 16: 'crowbar', 17: 'gas mask', 18: 'wrench', 19: 'video tape', 20: 'gloves',
      21: 'weedkiller', 22: 'grenade', 23: 'explosives', 24: 'silencer', 25: 'wire cutters'}
VT = {1: 'car', 2: 'van', 3: 'taxi', 4: 'police helicopter', 5: 'motorbike', 6: 'police car', 7: 'ambulance',
      8: 'meat wagon', 9: 'jeep', 10: 'sedan', 11: 'Wildcat van'}
VK = {0: 'unlocked', 1: 'red key', 2: 'blue key', 3: 'green key', 4: 'black key', 5: 'white key', 6: 'locked'}
CREATURE = {1: 'bat', 2: 'gargoyle', 3: 'Balrog', 4: 'Bane'}
BT = {0: 'barrel', 1: 'traffic cone', 2: 'bin', 3: 'burning barrel', 4: 'burning bin', 5: 'oil drum', 6: 'LOX drum'}

SHIPPED = ("FTutor1 Assault1 testdrive1a fight1 police1 testdrive2 fight2 police2 testdrive3 Bankbomb1 "
           "police3 Gangorder2 police4 bball2 wstores1 e3 westcrime1 carbomb1 botanicc Semtex AWOL1 mission2 "
           "park2 MIB skymiss2 factory1 estate2 Stealtst1 snow2 Gordout1 Baalrog3 Finale1 Gangorder1").split()


def load_sty(game):
    out, districts = {}, []
    lines = open(os.path.join(game, 'data', 'urban.sty'), encoding='latin-1').read().splitlines()
    in_d = False
    for l in lines:
        if l.startswith('[districts]'):
            in_d = True
            continue
        if in_d:
            if '=' in l:
                districts.append(l.split('=')[0].strip())
            continue
        if l.startswith('//') or not l.strip():
            continue
        parts = [p.strip() for p in l.split(':', 9)]
        if len(parts) < 10:
            continue
        oid, _, parent, _, _, _, dist, fn, title, brief = parts
        out[fn.rsplit('.', 1)[0].lower()] = dict(id=int(oid), parent=int(parent), district=int(dist), title=title,
                                                 brief=brief.replace('@', '\n'))
    return out, districts


def message_lines(path):
    if not path or not os.path.exists(path):
        return []
    text = open(path, encoding='latin-1').read()
    out = []
    for line in text.splitlines():
        m = re.match(r'\s*(\d+)\.\s*(\S.*)$', line)
        if m:
            out.append(m.group(2).strip())
    return out


def resolve(game, rel):
    cur = game
    for part in [p for p in re.split(r'[\\/]', rel) if p]:
        if not os.path.isdir(cur):
            return None
        hit = [x for x in os.listdir(cur) if x.lower() == part.lower()]
        if not hit:
            return None
        cur = os.path.join(cur, hit[0])
    return cur


class Mission:
    def __init__(self, name, m, goals, notes):
        self.name, self.m, self.goals, self.notes = name, m, goals, notes or {}
        self.eps = {e['index']: e for e in m['event_points'] if e.get('used', 1)}
        self.names = {int(k): v for k, v in self.notes.get('names', {}).items()}
        self.player_pts = [i for i, e in self.eps.items() if e['type'] == 'create_player']

    # ---- labels -------------------------------------------------------------
    def who(self, p):
        if p in self.names:
            return self.names[p]
        e = self.eps.get(p)
        if not e:
            return f'#{p}'
        if e['type'] == 'create_player':
            return PLAYER.get(e['data'][0], 'the player')
        if e['type'] == 'create_enemies':
            t = ET.get(e['data'][0] & 0xffff, 'someone')
            return f'{t[0].upper() + t[1:]} #{p}'
        if e['type'] == 'create_creature':
            return CREATURE.get(e['data'][0], 'creature') + f' #{p}'
        if e['type'] == 'create_vehicle':
            return f"the {VT.get(e['data'][0], 'vehicle')} #{p}"
        if e['type'] == 'create_item':
            return f"the {IT.get(e['data'][0], 'item')} #{p}"
        if e['type'] == 'create_barrel':
            return f"the {BT.get(e['data'][0], 'barrel')} #{p}"
        if e['type'] in ('message', 'conversation'):
            t = (e.get('text') or '').split('|')[0].strip()
            return f'"{t[:40]}{"..." if len(t) > 40 else ""}"'
        if e['type'] == 'nav_beacon':
            return f"beacon \"{e.get('text', '').strip()}\""
        return f"#{p} ({e['type'].replace('_', ' ')})"

    def speaker(self, e):
        d = e['data']
        if e['trigger'] == 'person_used' and e.get('ep_ref'):
            return self.who(e['ep_ref'])
        s = d[2]
        # A line set off by someone spotting the player is theirs to say, whoever the point
        # names: the Wildcats' threats are written that way.
        if s == 0 and e['trigger'] == 'person_seen' and e.get('ep_ref') and e['ep_ref'] not in self.player_pts:
            return self.who(e['ep_ref'])
        if s == 0:
            return 'Despatch (radio)'
        if s == 0xffff:
            return 'Street sign'
        if s == 0xfffe:
            return 'Instructions'
        return self.who(s)

    def when(self, e, depth=0):
        t, r, rb, rad = e['trigger'], e.get('ep_ref') or 0, e.get('ep_ref_bool') or 0, e.get('radius') or 0
        W = lambda p: self.who(p)
        if depth > 2:
            return 'earlier events'
        simple = lambda p: p in self.eps and self.eps[p]['type'] in ('simple', 'camera_waypoint', 'create_camera', 'create_target', 'kill_waypoint', 'increment', 'adjust_enemy', 'bonus_points', 'reset_counter', 'group_life', 'group_death')
        sub = lambda p: self.when(self.eps[p], depth + 1) if simple(p) else W(p)
        after = lambda p: self.when(self.eps[p], depth + 1) if simple(p) else f'after {W(p)}'
        d = {
            'none': 'at the start',
            'dependency': after(r),
            'radius': 'when the player gets near',
            'timer': f'{rad / 100:.0f}s into the mission' if rad else 'on a timer',
            'countdown': f'{rad / 100:.0f}s later ({after(r)})',
            'visible_countdown': f'on a {rad / 100:.0f}s countdown ({after(r)})',
            'boolean_and': f'once both: {sub(r)}; and {sub(rb)}',
            'boolean_or': f'once either: {sub(r)}; or {sub(rb)}',
            'killed': f'when {W(r)} ' + {'create_vehicle': 'is wrecked', 'create_barrel': 'blows up', 'create_creature': 'is killed'}.get(self.eps.get(r, {}).get('type'), 'is down'),
            'killed_not_arrested': f'when {W(r)} is killed (not arrested)',
            'person_arrested': f'when {W(r)} is arrested',
            'half_dead': f'when {W(r)} is down to half health',
            'group_dead': 'when the whole group is down',
            'person_used': f'on talking to {W(r)}',
            'player_uses_radius': 'on pressing action here',
            'conversation_over': f'after the conversation {W(r)}' if r in self.eps else 'after a conversation',
            'person_seen': f'when {W(rb)} is seen by {W(r)}' if rb else f'when {W(r)} is seen',
            'cuboid': 'on entering an area',
            'enemy_radius': f'when {W(r)} gets near',
            'thing_radius_dir': f'when {W(r)} gets near',
            'move_radius_dir': 'when the player moves through',
            'counter': 'when a count is reached',
            'person_in_vehicle': f'when {W(r)} is in {W(rb)}' if rb else f'when {W(r)} gets in a vehicle',
            'specific_item_held': f'on picking up {W(r)}',
            'item_held': 'on picking up an item',
            'random': 'at random',
            'darci_grabbed': 'when Darci is grabbed',
            'punched_and_kicked': 'after punching and kicking',
            'player_fires_gun': 'when the player fires a gun',
            'prim_damaged': 'when a prop is damaged',
            'tripwire': 'on a tripwire',
            'crime_rate_above': 'when the crime rate rises',
            'crime_rate_below': 'when the crime rate falls',
        }.get(t, t.replace('_', ' '))
        if e.get('flags', 0) & 1 and t not in ('none',):
            d = 'unless ' + d
        return d

    # ---- ordering -------------------------------------------------------------
    def depths(self):
        memo = {}

        def depth(i, seen):
            if i in memo:
                return memo[i]
            if i in seen or i not in self.eps:
                return 0
            e = self.eps[i]
            refs = [x for x in (e.get('ep_ref'), e.get('ep_ref_bool')) if x]
            if e['trigger'] in ('killed', 'person_arrested', 'half_dead', 'person_seen', 'enemy_radius',
                                'person_used', 'specific_item_held', 'thing_radius_dir', 'person_in_vehicle',
                                'killed_not_arrested'):
                # These name the thing watched; it existing comes first.
                pass
            v = 1 + max((depth(r, seen | {i}) for r in refs), default=-1)
            memo[i] = v
            return v

        return {i: depth(i, frozenset()) for i in self.eps}

    # ---- sections ---------------------------------------------------------------
    def people(self):
        speakers = set()
        for e in self.eps.values():
            if e['type'] == 'message':
                s = e['data'][2]
                if e['trigger'] == 'person_used' and e.get('ep_ref'):
                    speakers.add(e['ep_ref'])
                elif s not in (0, 0xffff, 0xfffe):
                    speakers.add(s)
            if e['type'] == 'conversation':
                speakers.update([e['data'][1], e['data'][2]])
            if e['type'] == 'nav_beacon' and e['data'][1]:
                speakers.add(e['data'][1])
            if e['trigger'] in ('killed', 'person_arrested', 'half_dead', 'killed_not_arrested', 'person_used') and e.get('ep_ref'):
                speakers.add(e['ep_ref'])
        named, crowd = [], Counter()
        for i, e in sorted(self.eps.items()):
            if e['type'] != 'create_enemies':
                continue
            d = e['data']
            et, n = d[0] & 0xffff, max(1, d[0] >> 16)
            ai = d[5] & 0xffff
            zone = (d[4] >> 8) & 0xff
            flags = []
            if zone & 16:
                flags.append('invulnerable')
            if zone & 32:
                flags.append('guilty')
            if (d[4] & 0xff) & 0x20:
                flags.append('stands still')
            if i in speakers or i in self.names:
                named.append((i, e, et, n, ai, flags))
            else:
                crowd[(ET.get(et, '?'), AI.get(ai, '?'))] += n
        return named, crowd

    def render(self, title, brief, info, synopsis, characters, unreleased):
        L = []
        m = self.m
        L.append(f"### {title}\n")
        L.append(f"`levels\\{self.name}.ucm`, map `{m.get('files', {}).get('map', '').split(B)[-1]}`"
                 + (f", district {info}" if info else '') + '\n')
        players = [PLAYER.get(self.eps[p]['data'][0], '?') for p in self.player_pts]
        transfers = [self.who(e['data'][0]) for e in self.eps.values() if e['type'] == 'transfer_player']
        if players:
            L.append(f"**Played as:** {', '.join(players)}" + (f", then {', '.join(transfers)}" if transfers else '') + '\n')
        if brief:
            L.append('**Briefing:**\n\n' + '\n'.join('> ' + x if x.strip() else '>' for x in brief.splitlines()) + '\n')
        if synopsis:
            L.append('**Story:** ' + synopsis + '\n')
        # characters
        named, crowd = self.people()
        if characters:
            L.append('**Characters:**\n')
            for c in characters:
                L.append(f'- {c}')
            L.append('')
        if named or crowd:
            L.append('<details><summary>Everyone the script makes</summary>\n')
            if named:
                L.append('| Point | Who | Type | Role | Notes |')
                L.append('|---|---|---|---|---|')
                for i, e, et, n, ai, flags in named:
                    who = self.names.get(i, '')
                    L.append(f"| {i} | {who} | {ET.get(et, '?')}{f' x{n}' if n > 1 else ''} | {AI.get(ai, '?')} | {', '.join(flags)} |")
                L.append('')
            if crowd:
                L.append('Others: ' + '; '.join(f'{c} {t} ({a})' for (t, a), c in sorted(crowd.items(), key=lambda x: -x[1])) + '.\n')
            L.append('</details>\n')
        # objects
        items = Counter()
        for e in self.eps.values():
            if e['type'] == 'create_item':
                items[IT.get(e['data'][0], f"item {e['data'][0]}")] += max(1, e['data'][1]) if e['data'][0] == 9 else 1
        vehicles = [(VT.get(e['data'][0], '?'), VK.get(e['data'][3], '?')) for e in self.eps.values() if e['type'] == 'create_vehicle']
        creatures = Counter(CREATURE.get(e['data'][0], '?') for e in self.eps.values() if e['type'] == 'create_creature')
        barrels = Counter(BT.get(e['data'][0], '?') for e in self.eps.values() if e['type'] == 'create_barrel')
        objs = []
        if items:
            objs.append('Items: ' + ', '.join(f'{k} x{v}' if v > 1 else k for k, v in items.most_common()))
        if vehicles:
            vc = Counter(f'{t} ({k})' for t, k in vehicles)
            objs.append('Vehicles: ' + ', '.join(f'{k} x{v}' if v > 1 else k for k, v in vc.most_common()))
        if creatures:
            objs.append('Creatures: ' + ', '.join(f'{k} x{v}' if v > 1 else k for k, v in creatures.most_common()))
        if barrels:
            objs.append('Street furniture: ' + ', '.join(f'{k} x{v}' if v > 1 else k for k, v in barrels.most_common()))
        other = Counter(e['type'] for e in self.eps.values())
        extras = [f'{n} {w}' for w, n in [('steam traps', other['create_trap']), ('mist banks', other['create_mist']),
                                            ('moving platforms', other['link_platform']), ('searchable props', other['make_searchable'])] if n]
        if extras:
            objs.append('Set pieces: ' + ', '.join(extras))
        if objs:
            L.append('**Objects:**\n')
            L += [f'- {o}' for o in objs]
            L.append('')
        g = self.goals or {}
        if g.get('badges') or g.get('sides'):
            L.append(f"**Badges:** {len(g.get('badges', []))}" + ('' if not g.get('sides') else '. **Side objectives:**') + '\n')
            for s in g.get('sides', []):
                L.append(f"- {s['label']} ({len(s['rewards'])} badge{'s' if len(s['rewards']) != 1 else ''})")
            if g.get('sides'):
                L.append('')
        beacons = [e for e in self.eps.values() if e['type'] == 'nav_beacon']
        if beacons:
            seen, bl = set(), []
            for e in sorted(beacons, key=lambda e: e['index']):
                t = (e.get('text') or '').strip()
                if t and t not in seen:
                    seen.add(t)
                    bl.append(t)
            L.append('**Beacons (places and people the player is sent to):** ' + ', '.join(f'"{b}"' for b in bl) + '\n')
        ends = []
        for e in sorted(self.eps.values(), key=lambda e: e['index']):
            if e['type'] == 'end_game_win':
                ends.append(f"- **Won** {self.when(e)}.")
            if e['type'] == 'end_game_lose':
                ends.append(f"- **Lost** {self.when(e)}.")
        if ends:
            L.append('**How it ends:**\n')
            L += ends
            L.append('')
        # script
        dep = self.depths()
        lines = [e for e in self.eps.values() if e['type'] in ('message', 'conversation', 'bonus_points', 'sign') and e.get('text')]
        if lines:
            L.append('<details><summary>Script</summary>\n')
            for e in sorted(lines, key=lambda e: (dep[e['index']], e['index'])):
                when = self.when(e)
                if e['type'] == 'conversation':
                    a, b = e['data'][1], e['data'][2]
                    L.append(f"\n*Conversation ({when}):*\n")
                    for n, l in enumerate([x.strip() for x in e['text'].split('|') if x.strip()]):
                        L.append(f"> **{self.who(a if n % 2 == 0 else b)}:** {l}  ")
                    L.append('')
                else:
                    who = 'Bonus' if e['type'] == 'bonus_points' else ('Sign' if e['type'] == 'sign' else self.speaker(e))
                    text = ' '.join(x.strip() for x in e['text'].split('|') if x.strip())
                    L.append(f"- **{who}:** {text} *({when})*")
            L.append('\n</details>\n')
        return '\n'.join(L)


def main():
    ucall, goals_path, game, notes_path, out = sys.argv[1:6]
    draft = sys.argv[sys.argv.index('--draft') + 1] if '--draft' in sys.argv else None
    goals = json.load(open(goals_path, encoding='utf-8'))
    notes = json.load(open(notes_path, encoding='utf-8')) if os.path.exists(notes_path) else {}
    sty, districts = load_sty(game)
    missions = {}
    for f in os.listdir(ucall):
        if f.endswith('.json') and os.path.getsize(os.path.join(ucall, f)):
            missions[f[:-5]] = json.load(open(os.path.join(ucall, f), encoding='utf-8'))
    order = sorted([n for n in missions if n.lower() in sty], key=lambda n: sty[n.lower()]['id'])
    if draft:
        os.makedirs(draft, exist_ok=True)
        for n, m in missions.items():
            s = sty.get(n.lower(), {})
            mm = Mission(n, m, goals.get(n), notes.get('missions', {}).get(n))
            open(os.path.join(draft, n + '.md'), 'w', encoding='utf-8').write(
                mm.render(s.get('title', n), s.get('brief', ''), districts[s['district']] if s else '', '', [], n not in sty))
        return
    # Each mission gets a file of its own in scripts/ next to the overview; the overview
    # lists them with their story.
    scripts = os.path.join(os.path.dirname(os.path.abspath(out)), 'scripts')
    os.makedirs(scripts, exist_ok=True)
    L = [notes.get('intro', '')]
    def section(n, unreleased):
        m = missions[n]
        s = sty.get(n.lower(), {})
        mn = notes.get('missions', {}).get(n, {})
        title = mn.get('title') or s.get('title', n)
        dist = districts[s['district']] if s else mn.get('district', '')
        mm = Mission(n, m, goals.get(n), mn)
        text = mm.render(title, s.get('brief', '') or mn.get('brief', ''), dist, mn.get('synopsis', ''), mn.get('characters', []), unreleased)
        text = '#' + text[3:]  # "### title" -> "# title"
        group = 'unreleased mission' if unreleased else 'released mission'
        open(os.path.join(scripts, n + '.md'), 'w', encoding='utf-8').write(
            f"{text.split(chr(10), 1)[0]}\n\n*A{'n' if unreleased else ''} {group} of Urban Chaos. "
            f"Back to [the story overview](../STORY.md).*\n\n{text.split(chr(10), 1)[1].lstrip()}".rstrip() + '\n')
        name, _, rest = title.partition(' *(')
        entry = f"- **[{name}](scripts/{n.replace(' ', '%20')}.md)**" + (f" *({rest}" if rest else '')
        if dist:
            entry += f", {dist}"
        if mn.get('synopsis'):
            entry += f": {mn['synopsis']}"
        return entry
    L.append(notes.get('released_header', ''))
    L += [section(n, False) for n in order] + ['']
    L.append(notes.get('unreleased_header', ''))
    L += [section(n, True) for n in notes.get('unreleased_order', []) if n in missions] + ['']
    L.append(notes.get('outro', ''))
    # street talk
    files = defaultdict(list)
    for n in order + notes.get('unreleased_order', []):
        if n in missions:
            c = missions[n].get('files', {}).get('citsez', '')
            p = resolve(game, c) if c and c != '*.txt' else None
            if p:
                files[os.path.basename(p)].append(n)
    if files:
        L.append(notes.get('street_header', ''))
        for fn, ms in sorted(files.items()):
            if fn.lower() == 'citsez.txt':
                continue  # the shared file, listed below
            p = resolve(game, 'text' + B + fn)
            lines = message_lines(p)
            titles = []
            for x in ms:
                t = sty[x.lower()]['title'] if x.lower() in sty else f'`{x}` (unreleased)'
                if t not in titles:
                    titles.append(t)
            L.append(f"### {', '.join(titles)} (`text\\{fn}`)\n")
            L += [f'- {x}' for x in lines]
            L.append('')
        for fn, label in [('citsez.txt', 'Any mission without its own file (`text\\citsez.txt`)'), ('annoyed.txt', 'Once frightened (`text\\annoyed.txt`)')]:
            p = resolve(game, 'text' + B + fn)
            if p:
                L.append(f'### {label}\n')
                L += [f'- {x}' for x in message_lines(p)]
                L.append('')
    open(out, 'w', encoding='utf-8').write('\n'.join(L).rstrip() + '\n')


if __name__ == '__main__':
    main()

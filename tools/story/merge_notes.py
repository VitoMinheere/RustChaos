"""Merges notes/*.json and the header texts into notes.json for story_gen.py."""
import json, os, sys

here = os.path.dirname(os.path.abspath(__file__))
ucall = sys.argv[1]
notes = {'missions': {}}
for f in sorted(os.listdir(os.path.join(here, 'notes'))):
    if f.endswith('.json'):
        notes['missions'].update(json.load(open(os.path.join(here, 'notes', f), encoding='utf-8')))
notes['intro'] = open(os.path.join(here, 'notes', 'intro.md'), encoding='utf-8').read()

# Drafts take the names of their released version where the same point makes the same kind
# of person.
BASE = {'police1b': 'police1', 'bball1': 'bball2', 'wstores11': 'wstores1', 'wstoresD': 'wstores1',
        'wstores1D': 'wstores1', 'carbomb2': 'carbomb1', 'botanic1': 'botanicc', 'botanicb': 'botanicc',
        'memtex': 'Semtex', 'test3': 'Semtex', 'test3old': 'Semtex', 'snow1': 'snow2', 'snow2_test': 'snow2',
        'Acarry': 'AWOL1', 'Finale1b': 'Finale1', 'gang2': 'Gangorder2', 'Gang order1': 'Gangorder1',
        'gang1': 'Gangorder1', 'gang3': 'Gangorder2', 'Gang order3': 'Gangorder2', 'Transmit1': 'factory1'}


def people(name):
    m = json.load(open(os.path.join(ucall, name + '.json'), encoding='utf-8'))
    return {e['index']: e['data'][0] & 0xffff for e in m['event_points'] if e['type'] == 'create_enemies'}


for draft, base in BASE.items():
    d, b = notes['missions'].setdefault(draft, {}), notes['missions'].get(base, {})
    dp, bp = people(draft), people(base)
    names = dict(d.get('names', {}))
    for k, v in b.get('names', {}).items():
        i = int(k)
        if k not in names and i in dp and dp[i] == bp.get(i):
            names[k] = v
    d['names'] = names

notes['released_header'] = """## The released missions

In the menu's order (`urban.sty`): each mission unlocks the next, except that the training and street missions interleave and Mako's missions sit among Darci's. Assassin unlocks last, after Day Of Reckoning.
"""
notes['unreleased_header'] = """## Unreleased missions

The GOG install has 97 mission files; the menu lists 33. The rest are below.
- **The lost missions** come first: finished or nearly finished, and never shipped.
- **Early prototypes** follow, from when the villain was "Mr X".
- **Drafts of released missions** come last, kept for what differs.

Titles marked *unofficial* are made up here from what the mission is about. A draft uses the names of its released version where the same people appear.

Not included:
- **Test files with nothing to tell:** `cutstest`, `moo`, `smoketest`, `oldsmoke1`, `pickups`; `downtown`, `downtwn2` and `downtwn2b`, whose map is missing and whose dialogue is placeholder ("more tedium... it's a yawn a minute round here").
- **Sixteen older-format files this project can't read yet:** `army1`, `Assassin1`, `Rooftop1`, `supply1`, `upperclass1`, `wibble`, `ECTS3C` (likely the ECTS trade-show demo), `e7`, `bang`, `downight`, `four`, `test`, `three`, `three_n`, `timetest`, `tm3`.
- **"Breakout!" (`Album1.ucm`):** a Dreamcast-only secret mission, unlocked after Estate of Emergency, and not in the PC install.
"""
notes['unreleased_order'] = [
    'jung3', 'FreeCD1', 'jung1', 'BAALROG1', 'creds1', 'Capflag1',
    'Takeover1', 'Getaway1', 'Transmit1', 'subdock1', 'backalley1', 'estate1', 'Crimescene', 'police5',
    'southsideroads', 'train1', 'train11', 'testdrive1', 'fight3',
    'police1b', 'gang1', 'gang2', 'gang3', 'Gang order3', 'Gang order1', 'bball1', 'wstores11', 'wstoresD',
    'wstores1D', 'carbomb2', 'botanic1', 'botanicb', 'test3old', 'test3', 'memtex', 'Acarry', 'snow1',
    'snow2_test', 'Finale1b']
notes['street_header'] = """## Street talk

What passers-by say when spoken to: the city's own voice. A mission with a chatter file of its own uses it, otherwise `citsez.txt`; anyone frightened uses `annoyed.txt`.
"""
json.dump(notes, open(os.path.join(here, 'notes.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print(len(notes['missions']), 'missions with notes')

# Target UC *(test copy; `snow2_test`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\snow2_test.ucm`, map `snow2.iam`

**Played as:** Darci

**Story:** Target UC as shipped.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 19 | Launch workman | workman | gang member | guilty |
| 26 | Wildcat prophet | Wildcat (rasta, pistol) | guard |  |
| 34 | Launch workman | workman | guard | guilty |
| 89 | Launch workman | workman | civilian | guilty |
| 273 | Man In Black | Man In Black | Man In Black |  |
| 278 | Wildcat | Wildcat (rasta, pistol) | guard |  |

Others: 11 Wildcat (red) (guard); 4 Wildcat (grey) (guard); 4 Wildcat (rasta, pistol) (guard); 2 Man In Black (Man In Black); 1 Wildcat (rasta, pistol) (gang member); 1 Wildcat (grey) (gang member); 1 Wildcat (red) (gang member).

</details>

**Objects:**

- Items: mine x54, barrels x40, explosives x7, shotgun ammo x4, grenade x2, shotgun, AK-47
- Vehicles: Wildcat van (locked) x5
- Set pieces: 17 steam traps, 2 mist banks, 3 moving platforms

**Badges:** 11

**Beacons (places and people the player is sent to):** "Warhead", "Supplies"

**How it ends:**

- **Won** after "All missile sites destroyed!".
- **Lost** 8s later (after "Missile launch sequence initiated!").
- **Lost** 8s later (after "Missile launch sequence active!").
- **Lost** 8s later (after "Missile launch sequence initiated!").
- **Lost** after "You took FAR TOO LONG - UNION CITY IS DU...".
- **Lost** after "Supplies Exhausted.".

<details><summary>Script</summary>

- **Despatch (radio):** Missile destroyed! *(when a prop is damaged)*
- **Despatch (radio):** Missile Destroyed! *(when a prop is damaged)*
- **Despatch (radio):** Missile destroyed! *(when a prop is damaged)*
- **Despatch (radio):** Bane's warheads are primed for launch any time soon. Infiltrate and destroy all 3 rockets using the explosives from the village. *(at the start)*
- **Wildcat prophet:** STERN of the UCPD! Your DEATH has been FORETOLD! *(on entering an area)*
- **Launch workman:** INTRUDER!! *(when Darci is seen by Launch workman)*
- **Despatch (radio):** Missile launch sequence initiated! *(when Launch workman gets near)*
- **Despatch (radio):** Missile launch sequence initiated! *(when Launch workman gets near)*
- **Despatch (radio):** Missile launch sequence active! *(when Launch workman gets near)*
- **Despatch (radio):** HURRY! THE WARHEADS ARE PRIMED! *(360s later (at the start))*

*Conversation (on entering an area):*

> **Man In Black:** Burning rain is gonna fall on the WHOLE CITY - Jus' like Master Bane foresaw..!  
> **Wildcat:** I hear you brother. But the word is that we ain't been able to shake the shadows - the old man and the cop-scum..!  
> **Man In Black:** We'll take care of them. Nothing can get in our way - can a cop arrest a 10 Megaton Missile? Hahargh!  

- **Despatch (radio):** You took FAR TOO LONG - UNION CITY IS DUST!! *(on a 420s countdown (at the start))*
- **Launch workman:** INTRUDER! Get her! *(when Darci is seen by Launch workman)*
- **Launch workman:** INTRUDER! Kill her! *(when Darci is seen by Launch workman)*
- **Despatch (radio):** All missile sites destroyed! *(once both: once both: "Missile destroyed!"; and "Missile Destroyed!"; and once both: once both: "Missile destroyed!"; and "Missile Destroyed!"; and "Missile destroyed!")*
- **Despatch (radio):** Supplies Exhausted. *(once both: once both: once both: earlier events; and earlier events; and once both: earlier events; and earlier events; and once both: once both: earlier events; and earlier events; and once both: earlier events; and earlier events)*

</details>

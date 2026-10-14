# Target UC

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\snow2.ucm`, map `snow2.iam`, district Snow Plains

**Played as:** Darci

**Briefing:**

> Get this - the encrypted files you got revealed Bane and his money boys at Gaemin Research have been bankrolling the covert construction of missiles! We got three major construction sites here, and we need them taken out! Looks like the place is crawling with Wildcats and Bane's personal bodyguards, "The Fallen". Don't take any chances with these assholes, D'arci - the bodyguards may look like Wall-Street chumps but they're tougher than nails, so watch your back every step of the way.

**Story:** The decrypted files show Bane and his money men at Gaemin Research have been building missiles in secret. Three launch sites in the Snow Plains, guarded by Wildcats and Bane's personal bodyguards, 'the Fallen' (they look like Wall Street chumps). Darci takes explosives from the village and destroys all three rockets before the warheads are primed. Workmen start the launch sequence if they see her. A Man In Black boasts that 'burning rain' will fall on the city, as Master Bane foresaw.

**Characters:**

- **Darci Stern**
- **The Fallen**: Bane's bodyguards, the Men In Black
- **Launch workmen** (guilty)
- Wildcats who say her death was foretold

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 19 | Launch workman | workman | civilian | guilty, stands still |
| 26 | Wildcat prophet | Wildcat (rasta, pistol) | guard |  |
| 34 | Launch workman | workman | guard | guilty, stands still |
| 89 | Launch workman | workman | civilian | guilty, stands still |
| 273 | Man In Black | Man In Black | Man In Black |  |
| 278 | Wildcat | Wildcat (rasta, pistol) | guard |  |

Others: 12 Wildcat (red) (guard); 4 Wildcat (grey) (guard); 4 Wildcat (rasta, pistol) (guard); 2 Man In Black (Man In Black); 1 Wildcat (rasta, pistol) (gang member); 1 Wildcat (grey) (gang member); 1 Wildcat (red) (gang member).

</details>

**Objects:**

- Items: mine x54, barrels x40, explosives x7, shotgun ammo x4, shotgun, AK-47, AK-47 ammo
- Vehicles: Wildcat van (locked) x5
- Set pieces: 17 steam traps, 2 mist banks, 3 moving platforms

**Badges:** 11

**Beacons (places and people the player is sent to):** "Warhead", "Supplies"

**How it ends:**

- **Won** after "All missile sites destroyed!".
- **Lost** 4s later (after "Missile launch sequence initiated!").
- **Lost** 4s later (after "Missile launch sequence active!").
- **Lost** 4s later (after "Missile launch sequence initiated!").
- **Lost** after "You took FAR TOO LONG - UNION CITY IS DU...".
- **Lost** after "Supplies Exhausted.".

<details><summary>Script</summary>

- **Despatch (radio):** Missile destroyed! *(when a prop is damaged)*
- **Despatch (radio):** Missile Destroyed! *(when a prop is damaged)*
- **Despatch (radio):** Missile destroyed! *(when a prop is damaged)*
- **Despatch (radio):** Bane's warheads are primed for launch any time soon. Infiltrate and destroy all 3 rockets using the explosives from the village. *(at the start)*
- **Launch workman:** INTRUDER!! *(when Darci is seen by Launch workman)*
- **Despatch (radio):** Missile launch sequence initiated! *(when Launch workman gets near)*
- **Despatch (radio):** Missile launch sequence initiated! *(when Launch workman gets near)*
- **Despatch (radio):** Missile launch sequence active! *(when Launch workman gets near)*
- **Despatch (radio):** HURRY! THE WARHEADS ARE PRIMED! *(360s later (at the start))*

*Conversation (on entering an area):*

> **Man In Black:** Burning rain is gonna fall on the WHOLE CITY - Jus' like Master Bane foresaw..!  
> **Wildcat:** I hear you brother. But the word is that we ain't been able to shake the shadows - the old man and the cop-scum..!  
> **Man In Black:** We'll take care of them. Nothing can get in our way - can a cop arrest a 10 Megaton Missile? Hahargh!  

- **Wildcat prophet:** STERN of the UCPD! Your DEATH has been FORETOLD! *(when Darci is seen by Wildcat prophet)*
- **Despatch (radio):** You took FAR TOO LONG - UNION CITY IS DUST!! *(on a 420s countdown (at the start))*
- **Launch workman:** INTRUDER! Get her! *(when Darci is seen by Launch workman)*
- **Launch workman:** INTRUDER! Kill her! *(when Darci is seen by Launch workman)*
- **Despatch (radio):** All missile sites destroyed! *(once both: once both: "Missile destroyed!"; and "Missile Destroyed!"; and once both: once both: "Missile destroyed!"; and "Missile Destroyed!"; and "Missile destroyed!")*
- **Despatch (radio):** Supplies Exhausted. *(once both: once both: once both: earlier events; and earlier events; and once both: earlier events; and earlier events; and once both: once both: earlier events; and earlier events; and when #141 is down)*

</details>

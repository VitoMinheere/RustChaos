# Construction Site Siege *(the 'Demo map'; `FreeCD1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\FreeCD1.ucm`, map `freeoncd1.iam`

**Played as:** Darci

**Story:** The mission of the playable demo (`levels_demo` in supermap.cpp pairs it with the combat tutorial; the attract mode calls it 'Demo map'). The local Wildcat chapter has stormed a construction site and taken workers hostage: Darci has four minutes to take out every Wildcat and keep the hostages alive. A tramp who 'already PAID you cops this month' sets his boys on her. The install has its speech but not its textures.

**Characters:**

- **Darci Stern**
- **Hostage construction workers**
- **The tramp**: claims he pays off the cops
- The local Wildcat chapter

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 9 |  | Wildcat (red) | guard |  |
| 10 |  | Wildcat (grey) | guard |  |
| 11 |  | Wildcat (red) | guard |  |
| 12 |  | Wildcat (red) | guard |  |
| 15 |  | Wildcat (grey) | guard |  |
| 16 |  | Wildcat (grey) | guard |  |
| 17 |  | Wildcat (grey) | guard |  |
| 18 |  | Wildcat (grey) | guard |  |
| 21 |  | Wildcat (rasta) | guard |  |
| 67 | The tramp | tramp | civilian |  |
| 103 | Hostage worker | workman | civilian | stands still |
| 104 | Hostage worker | workman | civilian | stands still |
| 105 | Hostage worker | workman | civilian | stands still |
| 106 | Hostage worker | workman | civilian | stands still |
| 125 |  | civilian | civilian |  |
| 208 |  | Wildcat (grey) | guard |  |

Others: 7 Wildcat (grey) (guard); 5 Wildcat (grey) (assassin); 5 Wildcat (red) (guard); 3 Wildcat (rasta) (guard); 3 Wildcat (rasta) (assassin).

</details>

**Objects:**

- Items: barrels x50, mine x18, pistol ammo x6, health pack x5
- Vehicles: police car (unlocked), police helicopter (unlocked)
- Street furniture: bin x24

**Badges:** 12

**Beacons (places and people the player is sent to):** "Target"

**How it ends:**

- **Won** when the whole group is down.
- **Won** when the whole group is down.
- **Lost** once either: once either: "Hostage down!"; or "Hostage down!"; or once either: "Hostage down!"; or "Hostage down!".
- **Lost** on a 240s countdown (at the start).

<details><summary>Script</summary>

- **Despatch (radio):** You killed him you PUTZ!! Didn't they teach you how to ARREST folk at the academy?? *(when The tramp is killed (not arrested))*
- **Despatch (radio):** Good arrest! *(when The tramp is arrested)*
- **Despatch (radio):** Hostage down! *(when Hostage worker is down)*
- **Despatch (radio):** Hostage down! *(when Hostage worker is down)*
- **Despatch (radio):** Hostage down! *(when Hostage worker is down)*
- **Despatch (radio):** Hostage down! *(when Hostage worker is down)*
- **Despatch (radio):** The local Wildcat chapter have stormed the Construction site and taken some workers hostage. You have FOUR minutes to take out ALL the Wildcats. We want those hostages ALIVE, Stern! *(at the start)*
- **Despatch (radio):** The Wildcats have spread out all over this section of town. Use any means you see fit in order to get that Wildcats scum in the bag! Be careful, Officer! *(at the start)*
- **Despatch (radio):** You MUST ensure the safety of any construction workers at the site...Good Luck! *(at the start)*
- **Civilian #125:** Beat it, copper! *(on talking to Civilian #125)*

*Conversation (once both: on entering an area; and when The tramp gets near):*

> **The tramp:** Hey YOU! Get the hell off my back yard!  
> **Darci:** Sir I ain't got time to argue. This is  Police business Im on!  
> **The tramp:** Yeah? I already PAID you cops this month - GET HER BOYS!  


</details>

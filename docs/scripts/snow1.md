# Target UC *(draft; `snow1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\snow1.ucm`, map `snow1.iam`

**Played as:** Darci

**Story:** An earlier Target UC on another snow map, with a motorbike.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 26 | Wildcat prophet | Wildcat (rasta, pistol) | guard |  |

Others: 15 Wildcat (rasta, pistol) (guard); 2 Wildcat (rasta) (guard); 1 civilian (guard); 1 civilian (civilian).

</details>

**Objects:**

- Items: mine x37, barrels x37, explosives x8, shotgun ammo x4, grenade x2, shotgun
- Vehicles: motorbike (unlocked) x3
- Set pieces: 3 moving platforms

**Badges:** 10

**How it ends:**

- **Won** after "All missile sites destroyed.".
- **Lost** 8s later (after "Missile launch sequence initiated").
- **Lost** 8s later (after "Missile launch sequence active").
- **Lost** 8s later (after "Missile launch sequence inititiated").

<details><summary>Script</summary>

- **Despatch (radio):** Missile destroyed. *(when a prop is damaged)*
- **Despatch (radio):** Missile Destroyed *(when a prop is damaged)*
- **Despatch (radio):** Missile destroyed *(when a prop is damaged)*
- **Civilian #34:** Alert, alert. *(when Darci is seen by Civilian #34)*
- **Despatch (radio):** Missile launch sequence inititiated *(when Civilian #34 gets near)*
- **Despatch (radio):** Missile launch sequence initiated *(when Wildcat (rasta) #19 gets near)*
- **Despatch (radio):** Missile launch sequence active *(when Civilian #89 gets near)*
- **Despatch (radio):** All missile sites destroyed. *(once both: once both: #49 (visual effect); and #63 (visual effect); and once both: once both: #49 (visual effect); and #63 (visual effect); and #55 (visual effect))*

</details>

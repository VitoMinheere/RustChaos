# Summoning Stones *(unofficial title; `jung1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\jung1.ucm`, map `jung1.iam`

**Played as:** Darci

**Story:** A second jungle level whose map is missing from the install (only `jung1bak.iam` survives). Darci destroys two summoning stones, then gets back before three waves of reinforcements arrive. Pressing action somewhere announces 'Extra multiplayer map available.'

**Characters:**

- **Darci Stern**
- Seventeen armed Wildcats

<details><summary>Everyone the script makes</summary>

Others: 17 Wildcat (rasta, pistol) (guard).

</details>

**Objects:**

- Items: pistol ammo x3, explosives x3, grenade, key, health pack

**How it ends:**

- **Won** once both: when the player gets near; and "Now get back before reenforcments arrive".

<details><summary>Script</summary>

- **Despatch (radio):** Extra multiplayer map available. *(on pressing action here)*
- **Despatch (radio):** Summoning stone destroyed *(when a prop is damaged)*
- **Despatch (radio):** Summoning stone destroyed *(when a prop is damaged)*
- **Despatch (radio):** Now get back before reenforcments arrive *(once both: "Summoning stone destroyed"; and "Summoning stone destroyed")*
- **Despatch (radio):** 1st wave of reenforcments arrived *(30s later (after "Now get back before reenforcments arrive"))*
- **Despatch (radio):** 2nd wave of reenforcements arrived *(60s later (after "Now get back before reenforcments arrive"))*
- **Despatch (radio):** 3rd wave of reenforcements arrived *(120s later (after "Now get back before reenforcments arrive"))*

</details>

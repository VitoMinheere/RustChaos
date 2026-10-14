# Driving Bronze

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\testdrive1a.ucm`, map `oval1.iam`, district Oval Circuit

**Played as:** Darci

**Briefing:**

> Complete 3 laps within the allotted time.

**Story:** Three laps of the oval in a squad car inside 90 seconds, avoiding cones and oil drums. The instructor starts the watch when Darci gets in. Running over the instructor, the other cops or the contractors fixing the station heating ends it.

**Characters:**

- **Darci Stern**
- **Driving instructor**
- Four cops watching
- Two contractors fixing the heating

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 45 | Driving instructor | cop | cop |  |
| 72 | Cop | cop | cop |  |
| 141 | Cop | cop | cop |  |
| 142 | Cop | cop | cop |  |
| 149 | Cop | cop | cop |  |
| 152 | Contractor | workman | civilian |  |
| 153 | Contractor | workman | civilian |  |

</details>

**Objects:**

- Vehicles: police car (unlocked)
- Street furniture: traffic cone x55, oil drum x33

**How it ends:**

- **Lost** when the police car #2 is wrecked.
- **Lost** after "TOO SLOW!".
- **Won** after #70 (stall car).
- **Lost** once either: "Are you nuts?"; or "Theres been a fatality. Training cancel...".
- **Lost** when Cop is down.
- **Lost** when Cop is down.
- **Lost** when Cop is down.
- **Lost** when Cop is down.
- **Lost** after "What you doin' to our contractors?".

<details><summary>Script</summary>

- **Driving instructor:** Okay rookie listen up!  Take the squad car 3 times around the course within the time limit. *(at the start)*
- **Driving instructor:** I'll start the watch as soon as you get into the car. Now get to it! *(at the start)*
- **Driving instructor:** Get in the car an' gimme 3 laps.. ON THE DOUBLE! *(on talking to Driving instructor)*
- **Despatch (radio):** 3 LAPS COMPLETE *(when a count is reached)*
- **Despatch (radio):** 2 LAPS COMPLETE *(when a count is reached)*
- **Despatch (radio):** 1 LAP COMPLETE *(when a count is reached)*
- **Despatch (radio):** Theres been a fatality. Training cancelled. *(when Driving instructor is killed (not arrested))*
- **Despatch (radio):** Are you nuts? *(when Driving instructor is arrested)*
- **Cop:** Ain't you supposed to be driving'? *(on talking to Cop)*
- **Cop:** Rookie you stink, but keep trying.. *(on talking to Cop)*
- **Cop:** Your car ain't up here, Rookie! *(on talking to Cop)*
- **Cop:** C'mon Stern, I wanna see you drive! *(on talking to Cop)*
- **Contractor:** Hi there officer, we're just fixin your busted heating system. *(on talking to Contractor)*
- **Contractor:** Howdy.. the jobs goin' well. *(on talking to Contractor)*
- **Driving instructor:** TOO SLOW! *(on a 90s countdown (when Darci is in the police car #2))*
- **Despatch (radio):** What you doin' to our contractors? *(once either: when Contractor is down; or when Contractor is down)*

</details>

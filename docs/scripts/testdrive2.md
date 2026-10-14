# Driving Silver

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\testdrive2.ucm`, map `road4_2.iam`, district Advanced Circuit

**Played as:** Darci

**Briefing:**

> Complete 2 laps in 120 seconds.

**Story:** Two laps of the intermediate course in 120 seconds, with a reverse park into a marked bay on each lap. Hurting or arresting the training officer ends it.

**Characters:**

- **Darci Stern**
- **Training officer**
- A second cop by the track

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 198 | Training officer | cop | cop |  |
| 251 | Cop | cop | cop |  |

</details>

**Objects:**

- Vehicles: police car (unlocked)
- Street furniture: traffic cone x162, barrel x26

**How it ends:**

- **Lost** when the police car #3 is wrecked.
- **Lost** once either: "Theres been an accident - the training ..."; or "What the hell you doin'? Didn't you go t...".
- **Lost** after "WAAY too slow!".
- **Won** after "Good work Stern!".

<details><summary>Script</summary>


*Conversation (at the start):*

> **Training officer:** Okay Stern, your doin' good. Heres what I'd like you to do next..  

- **Despatch (radio):** Theres been an accident - the training officer has been killed! *(when Training officer is killed (not arrested))*
- **Despatch (radio):** What the hell you doin'? Didn't you go through your "Basic Arrest Procedures" course? Get out of here Rookie! *(when Training officer is arrested)*
- **Despatch (radio):** At one point during a lap you are required to reverse park into the allotted bay and come to a full stop. *(at the start)*
- **Despatch (radio):** ..once again, avoid all cones. The time will begin when you get into the vehicle. *(at the start)*
- **Despatch (radio):** Back the car into the space to finish. *(when a count is reached)*
- **Training officer:** What are you waitin' for? *(on talking to Training officer)*
- **Despatch (radio):** ...make 2 laps of the intermediate course within the time limit. *(after the conversation "Okay Stern, your doin' good. Heres what...")*
- **Despatch (radio):** Good work Stern! *(once both: when the police car #3 gets near; and "Back the car into the space to finish.")*
- **Despatch (radio):** WAAY too slow! *(on a 120s countdown (when Darci is in the police car #3))*
- **Cop:** Get back in the car Stern! *(on talking to Cop)*
- **Despatch (radio):** Good stop...GO, GO, GO!!! *(once both: when the police car #3 gets near; and once both: earlier events; and earlier events)*
- **Despatch (radio):** 1 lap left. *(once both: when the police car #3 gets near; and once both: earlier events; and earlier events)*

</details>

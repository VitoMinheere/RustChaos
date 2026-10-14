# Driving Basics *(early Driving Bronze; `testdrive1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\testdrive1.ucm`, map `road4.iam`

**Played as:** Darci

**Story:** Two laps of the basic course, clockwise, in one minute, avoiding cones.

**Characters:**

- **Darci Stern**
- **Training officer**

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 199 | Training officer | cop | cop | stands still |
| 242 |  | cop | cop | stands still |

</details>

**Objects:**

- Vehicles: van (locked) x2, police car (unlocked), meat wagon (locked), jeep (locked)
- Street furniture: traffic cone x219

**How it ends:**

- **Lost** once either: "There's been an accident! The training o..."; or "What the hell are you playin' at?".
- **Won** after "Basic training complete!".
- **Lost** after "You didn't make the grade.But hell, give...".
- **Lost** after "Your cars blown!".

<details><summary>Script</summary>


*Conversation (at the start):*

> **Training officer:** Okay Stern, heres what ya gotta do...  

- **Despatch (radio):** Your cars blown! *(when the police car #2 is wrecked)*
- **Despatch (radio):** Go clockwise around the basic course and avoid hitting the cones. *(at the start)*
- **Despatch (radio):** You have 1 minute to complete 2 laps, starting when you get into the vehicle. *(at the start)*
- **Despatch (radio):** There's been an accident! The training officer has been killed! *(when Training officer is killed (not arrested))*
- **Despatch (radio):** You didn't make the grade.But hell, give it another shot an' you might get lucky! *(when a count is reached)*
- **Despatch (radio):** Basic training complete! *(at the start)*
- **Despatch (radio):** What the hell are you playin' at? *(when Training officer is arrested)*
- **Despatch (radio):** Waypoint hit. *(when the police car #2 gets near)*
- **Despatch (radio):** Waypoint hit. *(when the police car #2 gets near)*
- **Despatch (radio):** Waypoint hit. *(when the police car #2 gets near)*
- **Despatch (radio):** Well done Rookie. I guess you progress to the next course! *(once both: when the police car #2 gets near; and when a count is reached)*
- **Despatch (radio):** For this simple test, all you need to do is get in the car an' take it for a spin. *(after the conversation "Okay Stern, heres what ya gotta do...")*
- **Despatch (radio):** 1 lap left. *(once both: when the police car #2 gets near; and once both: earlier events; and earlier events)*
- **Despatch (radio):** STOP within the coned area. *(once both: when the police car #2 gets near; and "1 lap left.")*
- **Despatch (radio):** counter up by one *(once both: earlier events; and earlier events)*
- **Despatch (radio):** Theres been an accident! The training officer has been killed! *(when Cop #242 is killed (not arrested))*

</details>

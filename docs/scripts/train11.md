# Driving School *(unofficial title; `train11`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\train11.ucm`, map `training1.iam`

**Played as:** Darci

**Story:** A variant of Basic Training with a driving-school section.

**Characters:**

- **Darci Stern**
- Training officers

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 2 |  | cop | cop | stands still |
| 17 |  | cop | cop |  |
| 19 |  | Wildcat (rasta) | training dummy |  |
| 140 |  | cop | cop | stands still |

Others: 16 cop (cop); 3 civilian (stands about); 3 Wildcat (red) (guard); 1 Wildcat (grey) (guard).

</details>

**Objects:**

- Items: barrels x57, shotgun, pistol
- Vehicles: taxi (unlocked) x2, van (unlocked), motorbike (unlocked)
- Set pieces: 5 moving platforms

**Badges:** 1

**How it ends:**

- **Won** after the conversation "Well done, you have completed the course...".
- **Lost** after "You have assaulted an officer.".
- **Lost** after "You have assaulted an officer!".

<details><summary>Script</summary>

- **Despatch (radio):** Driving School *(on entering an area)*

*Conversation (on entering an area):*

> **Cop #2:** Well done officer, you have successfully completed basic training.  

- **Despatch (radio):** You hit a laser consider yourself dead. *(on a tripwire)*
- **Despatch (radio):** Hit a laser..! *(on a tripwire)*
- **Despatch (radio):** You hit a laser fool... *(on a tripwire)*
- **Despatch (radio):** Laser beam broken...Not good. *(on a tripwire)*
- **Despatch (radio):** Laser broken...crazy fool. *(on a tripwire)*
- **Despatch (radio):** To talk to people press your ACTION key. *(when the player gets near)*
- **Despatch (radio):** Tripwire activated *(on a tripwire)*
- **Despatch (radio):** Tripwire Activated *(on a tripwire)*
- **Despatch (radio):** Well done..all borgs killed. *(when the whole group is down)*
- **Despatch (radio):** To climb the ladder just walk toward it. *(when the player gets near)*
- **Despatch (radio):** Follow path around corner. *(on pressing action here)*
- **Despatch (radio):** Negoitiate the jumps ahead. Be careful, they get harder. *(when the player gets near)*
- **Despatch (radio):** Turn left and continue around the jumps. *(on pressing action here)*
- **Despatch (radio):** Shooting Range. *(on entering an area)*
- **Despatch (radio):** To get in a vehicle, approach and press action. *(on entering an area)*

*Conversation (on entering an area):*

> **Cop #2:** Well done, you have completed the course successfully, the streets of Union City however, are far more dangerous.  

- **Despatch (radio):** Collecting shields reduces the crime rate and reveals secrets. *(on entering an area)*
- **Despatch (radio):** To activate the lift, use the switch to the right. *(on entering an area)*

*Conversation (on talking to Cop #17):*

> **Cop #17:** Hello officer.  
> **Darci:** Hi, where am I supposed to go now?  
> **Cop #17:** Follow me, I'll show you.  

- **Despatch (radio):** You may now choose the shooting range, driving school or hand to hand arena. *(after the conversation "Well done officer, you have successfully...")*
- **Despatch (radio):** Welcome to officer training, Stern. Here you'll learn to jump, climb and shoot like a Union City Officer. Now get goin'! *(after Darci)*
- **Despatch (radio):** Loading lift activated *(on pressing action here)*
- **Despatch (radio):** You have assaulted an officer. *(when Cop #2 is down)*
- **Despatch (radio):** You have assaulted an officer! *(when Cop #17 is down)*
- **Despatch (radio):** Move your BUTT rookie - our guys are all around this course, just WAITIN' for you to mess up. If you do, you're OUT! Hwahwahwa... *(on entering an area)*
- **Cop #140:** Move it, greenhorn! *(on talking to Cop #140)*
- **Despatch (radio):** Get over the jumps and I'll meet you later. *(once both: when Cop #17 gets near; and on talking to Cop #17)*
- **Despatch (radio):** Advanced course available. *(after #83 (activate prim))*

</details>

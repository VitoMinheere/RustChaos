# Basic Training *(unofficial title; `train1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\train1.ucm`, map `training1.iam`

**Played as:** Darci

**Story:** An early training course on its own map, with laser tripwires ('You hit a laser, consider yourself dead'), a motorbike, and a lesson in talking to people with the action key.

**Characters:**

- **Darci Stern**
- Training officers

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 2 |  | cop | cop | stands still |
| 17 |  | cop | cop |  |
| 80 |  | cop | cop |  |
| 160 |  | cop (pistol) | cop |  |

Others: 7 cop (cop); 3 Wildcat (red) (guard); 1 Wildcat (grey) (guard); 1 cop (training dummy).

</details>

**Objects:**

- Items: barrels x57, pistol x3, health pack x2, shotgun
- Vehicles: taxi (unlocked) x2, van (unlocked), motorbike (unlocked)
- Street furniture: barrel x3
- Set pieces: 5 moving platforms

**Badges:** 1

**How it ends:**

- **Lost** after "You've broken to many laser beams!".
- **Won** after the conversation "Well done, you have completed the course...".
- **Lost** after "You have assaulted an officer.".
- **Lost** after "You have assaulted an officer!".
- **Lost** after "You have assaulted an officer.".

<details><summary>Script</summary>


*Conversation (on entering an area):*

> **Cop #2:** Well done officer, you have successfully completed basic training.  

- **Despatch (radio):** You hit a laser consider yourself dead. *(on a tripwire)*
- **Despatch (radio):** Hit a laser..! *(on a tripwire)*
- **Despatch (radio):** You hit a laser fool... *(on a tripwire)*
- **Despatch (radio):** Laser beam broken...Not good. *(on a tripwire)*
- **Despatch (radio):** Laser broken...crazy fool. *(on a tripwire)*
- **Despatch (radio):** To talk to people press your ACTION key. *(when the player gets near)*
- **Despatch (radio):** Tripwire activated *(on a tripwire)*
- **Despatch (radio):** Tripwire activated *(on a tripwire)*
- **Despatch (radio):** To climb the ladder just walk toward it. *(when the player gets near)*
- **Despatch (radio):** Negoitiate the jumps ahead. Be careful, they get harder so you will need to sprint using your action key. *(when the player gets near)*
- **Despatch (radio):** Shooting Range. *(on entering an area)*
- **Despatch (radio):** To get in a vehicle, approach and press action. *(on entering an area)*

*Conversation (on entering an area):*

> **Cop #80:** Well done, you have completed the course successfully, the streets of Union City however, are far more dangerous.  

- **Despatch (radio):** Collecting shields reduces the crime rate and reveals secrets. *(on entering an area)*
- **Despatch (radio):** To activate the lift, use the switch to the right. *(on entering an area)*
- **Despatch (radio):** Pick up a pistol using action, and destroy all three targets. *(on entering an area)*
- **Despatch (radio):** If you have been injured at any time, you may pick up health packs to increase your energy. Press action to pick up the health pack. *(on entering an area)*

*Conversation (on talking to Cop #17):*

> **Cop #17:** Hello officer.  
> **Darci:** Hi, where am I supposed to go now?  
> **Cop #17:** Follow me, I'll show you.  

- **Despatch (radio):** You may now choose the shooting range, driving school or hand to hand arena. *(after the conversation "Well done officer, you have successfully...")*
- **Despatch (radio):** Use this first area to practice moving around, then go and talk to the training officer who's just in front of you, he'll let you know what to do next. *(at the start)*
- **Despatch (radio):** Welcome to the assault course Stern. Here you will learn all the basic techniques which you will need to master in order to survive in Union City. *(at the start)*
- **Despatch (radio):** You've broken to many laser beams! *(when a count is reached)*
- **Despatch (radio):** Loading lift activated *(on pressing action here)*
- **Despatch (radio):** You have assaulted an officer. *(when Cop #2 is down)*
- **Despatch (radio):** You have assaulted an officer! *(when Cop #17 is down)*
- **Despatch (radio):** Well done, now negotiate the following set of even harder jumps. *(once either: on entering an area; or on entering an area)*
- **Despatch (radio):** You have assaulted an officer. *(when Cop (pistol) #160 is down)*

*Conversation (on talking to Cop (pistol) #160):*

> **Cop (pistol) #160:** Stern, you can use any of the vehicles to get use to the controls and driving styles, please take care of them, or the chief won't be happy.  

- **Despatch (radio):** Press your weapon select key to choose the gun. *(on picking up an item)*

*Conversation (once both: when Cop #17 gets near; and on talking to Cop #17):*

> **Cop #17:** Get over them jumps, I'll meet you later.  

- **Despatch (radio):** Well done..all targets destroyed. *(once both: once both: earlier events; and earlier events; and when the barrel #7 blows up)*
- **Despatch (radio):** Advanced course available. *(after #83 (activate prim))*

</details>

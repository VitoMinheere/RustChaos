# Driving Gold

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\testdrive3.ucm`, map `road4_3.iam`, district Advanced Circuit

**Played as:** Darci

**Briefing:**

> Complete 2 laps in 140 seconds.

**Story:** The advanced course in a meat wagon: two laps in 140 seconds, with two reverse parks per lap.

**Characters:**

- **Darci Stern**
- **Training officer**

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 90 | Training officer | cop | cop |  |

</details>

**Objects:**

- Vehicles: meat wagon (unlocked)
- Street furniture: traffic cone x257

**How it ends:**

- **Won** after "Excellent driving Stern!".
- **Lost** once either: "Theres been an accident - the training ..."; or "What have you done??".
- **Lost** after "Out of time!".
- **Lost** when the meat wagon #1 is wrecked.

<details><summary>Script</summary>


*Conversation (at the start):*

> **Training officer:** Okay Stern, this is it - the tough one..  

- **Despatch (radio):** What have you done?? *(when Training officer is arrested)*
- **Despatch (radio):** Theres been an accident - the training officer has been killed! *(when Training officer is killed (not arrested))*
- **Despatch (radio):** Reverse into the parking spot to finish, Rookie! *(when a count is reached)*
- **Despatch (radio):** There are 2 points at which you will have to reverse park in each lap. *(at the start)*
- **Despatch (radio):** The timer starts when you enter the truck. *(at the start)*
- **Despatch (radio):** Out of time! *(on a 140s countdown (when Darci is in the meat wagon #1))*
- **Despatch (radio):** Excellent driving Stern! *(once both: when the meat wagon #1 gets near; and "Reverse into the parking spot to finish,...")*
- **Despatch (radio):** Get around the advanced track as fast as possible! *(after the conversation "Okay Stern, this is it - the tough one..")*
- **Despatch (radio):** Good stop! Now GO! *(once both: when the meat wagon #1 gets near; and once both: earlier events; and earlier events)*
- **Despatch (radio):** Nice stop! *(once both: when the meat wagon #1 gets near; and once both: earlier events; and earlier events)*
- **Despatch (radio):** 1 lap left. *(once both: when the meat wagon #1 gets near; and once both: earlier events; and earlier events)*

</details>

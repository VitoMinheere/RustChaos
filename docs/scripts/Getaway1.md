# Mr X's Getaway *(unofficial title; `Getaway1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Getaway1.ucm`, map `tv1.iam`

**Played as:** Darci

**Story:** Prototype on the TV-centre map: 'Mr X and his bodyguards are making a hasty exit from the TV station.' Darci meets her police contact at Lexington and 12th and must intercept Mr X before his van gets away, last seen near York Park. Street signs name the area's junctions.

**Characters:**

- **Darci Stern**
- **Mr X**
- **Police contact**

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 131 | Mr X | civilian | gang member | stands still |
| 162 | Police contact | cop | cop |  |

Others: 34 civilian (civilian); 12 civilian (driver); 3 Wildcat (grey, AK-47) (bodyguard); 3 Wildcat (grey, AK-47) (guard); 2 cop (driver).

</details>

**Objects:**

- Items: AK-47
- Vehicles: car (locked) x11, car (unlocked) x7, taxi (unlocked) x3, van (unlocked) x3, van (locked) x2, police car (unlocked) x2

**Badges:** 10

**Beacons (places and people the player is sent to):** "Police contact"

**How it ends:**

- **Won** when Mr X is down.
- **Lost** after "Looks like he got away!".

<details><summary>Script</summary>

- **Despatch (radio):** Junction of Lexington and 12th Avenue *(on entering an area)*
- **Despatch (radio):** Junction Of Lexington and Gidani Street *(on entering an area)*
- **Despatch (radio):** Junction Of 12th and Durham Street *(on entering an area)*
- **Despatch (radio):** Junction Of Gidani And Durham Street. *(on entering an area)*
- **Despatch (radio):** Metal Works Factory. *(on entering an area)*
- **Despatch (radio):** Junction Of Gidani And Eastern Avenue. *(on entering an area)*
- **Despatch (radio):** Havalan auto parking. *(on entering an area)*
- **Despatch (radio):** gate open. *(on pressing action here)*
- **Despatch (radio):** Meet contact at corner of lexington and 12th. Check your nav map. *(at the start)*

*Conversation (on talking to Police contact):*

> **Darci:** Whats going on officer?  
> **Police contact:** Mr X and his bodyguards are making a hasty exit from the tv station, intercept him before he gets away.!  

- **Despatch (radio):** Looks like he got away! *(when the van #164 gets near)*
- **Despatch (radio):** Objective reported to be near York park. *(when Mr X gets near)*

</details>

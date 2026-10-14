# Farraday's Files *(early Estate of Emergency; `estate1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\estate1.ucm`, map `nestate0x.iam`

**Played as:** Darci

**Story:** An earlier take on Bane's estate: disarm the security from the pool house, rescue Mr Farraday, copy 'all information about the cult' from the main computer onto a disk, and hand it to Officer Jed at the main gates.

**Characters:**

- **Darci Stern**
- **Mr Farraday**: captive
- **Officer Jed**: Darci's superior on the job

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 41 | Mr Farraday | civilian | civilian |  |
| 105 | Officer Jed | cop (pistol) | cop |  |

Others: 12 Wildcat (rasta) (guard); 7 Wildcat (rasta, pistol) (assassin); 6 Wildcat (rasta, pistol) (guard); 6 cop (pistol) (cop); 4 cop (pistol) (cop driver); 1 Wildcat (rasta) (assassin).

</details>

**Objects:**

- Items: health pack x3, floppy disk, keycard
- Vehicles: police car (unlocked) x4

**Badges:** 10

**Beacons (places and people the player is sent to):** 

**How it ends:**

- **Won** after the conversation "Well done Darci, those files will be of ...".

<details><summary>Script</summary>


*Conversation (when the player gets near):*

> **Mr Farraday:** Thank god youve come, please get me out of this place.  
> **Darci:** Okay Mr Farraday, don't worry about a thing.  
> **Mr Farraday:** Oh thank you officer.  

- **Despatch (radio):** In cube. *(when the player gets near)*
- **Despatch (radio):** Security systems off. Meet your superior at the main gates to recieve disk! *(on pressing action here)*
- **Despatch (radio):** This is not the main computer. *(on pressing action here)*
- **Despatch (radio):** Mr Farraday has been killed. *(when Mr Farraday is down)*
- **Despatch (radio):** I'll be safe here...thanks. *(when Mr Farraday gets near)*
- **Despatch (radio):** ALERT ALERT..Intruder!! *(once either: when the player gets near; or when the player gets near)*
- **Despatch (radio):** This building contains the main computer. To get in, you will need to disarm the security measures. *(at the start)*
- **Despatch (radio):** Copying Files...Please Wait. *(on pressing action here)*
- **Despatch (radio):** The security systems can be disarmed by throwing the overide switch in the pool house. *(at the start)*

*Conversation (once both: when the player gets near; and Officer Jed):*

> **Darci:** Hi officer Jed, security systems are down.  
> **Officer Jed:** Good work Darci, now get to the main computer and install all information about the cult onto this file.  

- **Bonus:** Secondary objective complete. 200 points. *(after "I'll be safe here...thanks.")*
- **Despatch (radio):** Files Copied. *(on a 20s countdown (after "Copying Files...Please Wait."))*

*Conversation (once both: when the player gets near; and on picking up an item):*

> **Officer Jed:** Well done Darci, those files will be of vital importance to us.  
> **Darci:** Thanks.  


</details>

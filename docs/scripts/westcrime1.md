# Urban Shakedown

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\westcrime1.ucm`, map `disturb1.iam`, district West District

**Played as:** Darci

**Briefing:**

> You may have hit the Jackpot with that last collar, Stern. The big badass gang boss you brought in squealed like a stuck pig once O'Byrne and Kowalski went to work on him in the cell. Apparently the Wildcats got an itching to take over this part of Union City. We don't know why this area's so important to them, but it seems like the 'Cats are assembling a virtual gang-banger ARMY to sweep across the area. Get your butt on the streets an' give them all a proper UCPD welcome!

**Story:** The gang boss from the stadium squealed under O'Byrne and Kowalski: the Wildcats are massing an army to take West District. The station is attacked and Darci has three minutes to break every wave of Wildcats, Man In Black fighters among them, before the district falls. Officer Schwartz insists on coming along and must be kept alive.

**Characters:**

- **Darci Stern**
- **Officer Schwartz**: eager volunteer
- **O'Byrne** and **Kowalski** (mentioned): did the interrogation
- Waves of Wildcats and Man In Black fighters
- A loiterer and frightened residents

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 8 |  | Wildcat (grey) | Man In Black |  |
| 16 |  | Wildcat (grey) | Man In Black |  |
| 17 |  | Wildcat (grey) | Man In Black |  |
| 18 |  | Wildcat (grey) | Man In Black |  |
| 24 | Loiterer | civilian | civilian |  |
| 32 |  | Wildcat (grey) | Man In Black |  |
| 33 |  | Wildcat (grey) | Man In Black |  |
| 37 |  | Wildcat (rasta) | assassin |  |
| 38 |  | Wildcat (rasta) | assassin |  |
| 41 |  | Wildcat (red) | guard |  |
| 44 |  | Wildcat (grey) | Man In Black |  |
| 51 |  | Wildcat (red) | assassin |  |
| 53 |  | Wildcat (red) | assassin |  |
| 60 |  | Wildcat (red) | assassin |  |
| 61 |  | Wildcat (red) | assassin |  |
| 65 |  | Wildcat (red) | guard |  |
| 67 |  | Wildcat (red) | assassin |  |
| 72 |  | Wildcat (red) | assassin |  |
| 74 |  | Wildcat (rasta) | guard |  |
| 76 |  | Wildcat (grey) | Man In Black |  |
| 77 |  | Wildcat (grey) | Man In Black |  |
| 78 |  | Wildcat (grey) | Man In Black |  |
| 90 | Officer Schwartz | cop | cop |  |
| 94 |  | Wildcat (red) | Man In Black |  |
| 137 |  | Wildcat (rasta) | Man In Black |  |
| 160 |  | Wildcat (rasta) | guard |  |
| 163 |  | Wildcat (rasta) | assassin |  |
| 165 |  | Wildcat (rasta) | assassin |  |
| 167 |  | Wildcat (red) | guard |  |
| 231 |  | Wildcat (red) | guard |  |
| 232 |  | Wildcat (rasta) | guard |  |
| 233 |  | Wildcat (rasta) | guard |  |
| 289 | Resident | civilian | civilian |  |
| 290 | Resident | civilian | civilian |  |
| 291 | Resident | civilian | civilian |  |

Others: 4 cop (driver); 3 cop (a body (dies at once)).

</details>

**Objects:**

- Items: mine x10, health pack x10, pistol ammo x5, shotgun ammo, knife
- Vehicles: police car (locked) x8, police car (unlocked) x3, car (unlocked)
- Street furniture: bin x21

**Badges:** 12. **Side objectives:**

- Side objective (1 badge)
- TIME EXTENDED: New Targets Aquired (3 badges)

**Beacons (places and people the player is sent to):** "Schwarz", "Wild Cat"

**How it ends:**

- **Won** after "Wild Cats eliminated!".
- **Lost** after "Too late, the Wild Cats have overrun the...".

<details><summary>Script</summary>

- **Despatch (radio):** TIME EXTENDED: New Targets Acquired *(when the whole group is down)*
- **Despatch (radio):** TIME EXTENDED: New Targets Acquired *(when the whole group is down)*
- **Despatch (radio):** TIME EXTENDED: New Targets Acquired *(when the whole group is down)*
- **Despatch (radio):** Wild Cats eliminated! *(when the whole group is down)*
- **Despatch (radio):** TIME EXTENDED: New Targets Aquired *(when the whole group is down)*
- **Despatch (radio):** Stern, things here are goin' crazy! We just had the station attacked. *(at the start)*

*Conversation (on talking to Loiterer):*

> **Loiterer:** Hi wassup?  
> **Darci:** You've been hanging around here all night. Any particular reason?  
> **Loiterer:** Oh I'm just waiting for someone.  
> **Darci:** Smartass....  

- **Despatch (radio):** You arrested Schwarz! *(when Officer Schwartz is arrested)*

*Conversation (on talking to Officer Schwartz):*

> **Officer Schwartz:** Hey Stern!  
> **Darci:** Yeah?  
> **Officer Schwartz:** I'm coming with ya on this one.. the boys told me to stay back, but I want a piece!  
> **Darci:** Okay officer, whats your name?  
> **Officer Schwartz:** Schwartz, Officer Schwartz.  
> **Darci:** Okay Schwartz, lets go.  

- **Despatch (radio):** Schwarz has been wasted! *(when Officer Schwartz is killed (not arrested))*
- **Resident:** Get yo ass outta my face! *(on talking to Resident)*
- **Resident:** I ain't never seen crime like this! *(on talking to Resident)*
- **Resident:** You cops should be doin' somethin' about all this trouble! *(on talking to Resident)*
- **Despatch (radio):** We ain't got long until this place will be over-run. Get out there an' drive 'em back to the pit they came from! *(at the start)*
- **Despatch (radio):** ...we got Wildcats movin' in. Seems they want to take over this district! *(at the start)*
- **Despatch (radio):** Get out there and take down all Wildcat targets. *(at the start)*
- **Despatch (radio):** Too late, the Wild Cats have overrun the district! *(on a 180s countdown (after "We ain't got long until this place will ..."))*

</details>

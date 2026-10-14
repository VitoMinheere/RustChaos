# Auto-Destruct *(draft; `carbomb2`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\carbomb2.ucm`, map `gang1.iam`

**Played as:** Darci

**Story:** Auto-Destruct almost as shipped.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 18 | Bomb engineer | cop | cop | stands still |
| 19 | Bomb engineer | cop | cop | stands still |
| 20 | Bomb engineer | cop | cop | stands still |
| 47 | Cop | cop | cop | stands still |
| 51 | Doom-sayer | civilian | civilian |  |
| 53 | Bane-watcher | civilian | civilian |  |
| 54 | Puzzled citizen | civilian | civilian |  |
| 63 | Citizen | civilian | civilian |  |
| 64 | Conspiracy theorist | civilian | civilian |  |
| 65 | Citizen | civilian | civilian |  |
| 66 | Conspiracy theorist | civilian | civilian |  |
| 73 | Citizen | civilian | civilian |  |
| 74 | Citizen | civilian | civilian |  |
| 75 | Citizen | civilian | civilian |  |
| 101 | Bomb engineer | cop | cop | stands still |
| 102 | Bomb engineer | cop | cop | stands still |
| 103 | Bomb engineer | cop | cop | stands still |
| 147 | Bomber | Wildcat (red) | bodyguard |  |
| 148 | Bomber | Wildcat (red) | bodyguard |  |
| 174 | The Sarge | cop | cop | stands still |

Others: 18 Wildcat (grey) (guard); 7 Wildcat (red) (guard); 3 cop (cop); 2 Wildcat (rasta) (guard); 2 Wildcat (grey) (assassin).

</details>

**Objects:**

- Items: health pack
- Vehicles: meat wagon (locked) x6, car (locked) x6, taxi (locked) x5, sedan (unlocked) x2, van (locked) x2, police car (locked)
- Street furniture: bin x21, traffic cone x8
- Set pieces: 1 searchable props

**Badges:** 12

**Beacons (places and people the player is sent to):** "Car Bomb", "Car Bomb.", "Demo Squad 1", "Demo Squad 2", "Bomber", "Wild Cat Den"

**How it ends:**

- **Lost** when the sedan #82 is wrecked.
- **Lost** when #43 (simple) is down.
- **Lost** once either: once either: "You arrested one of the engineers! You A..."; or "Is that funny? Handcuffing the engineers..."; or once either: once either: "You arrested one of the engineers!"; or "You killed one of the engineers!"; or once either: "SCHMUCK! You killed one of the engineers..."; or "Stern, have you gone BUGGO?".
- **Lost** once either: once either: "STERN! You SHOT the engineer!"; or "What the - Stern, you just shot the ENGI..."; or once either: once either: "You killed one of the engineers! Shit  S..."; or "Arresting the engineers? Is that some so..."; or once either: "What the hell are you doing arresting th..."; or "You arrested one of the engineers! Is th...".
- **Lost** after "The car's blown!".
- **Lost** after "The car's blown!".
- **Lost** once either: "You killed him! Well, there goes a great..."; or "Stern, we needed those Terrorists ALIVE!...".
- **Won** after the conversation "You find anything?".
- **Won** after the conversation "Well? Come up with anything?".

<details><summary>Script</summary>

- **Despatch (radio):** Terrorists have planted a bomb in a brown Sedan outside the Bank. Get that machine back to the Bomb disposal squad NOW! *(at the start)*
- **Despatch (radio):** You killed one of the engineers! *(when Bomb engineer is killed (not arrested))*
- **Despatch (radio):** SCHMUCK! You killed one of the engineers! *(when Bomb engineer is killed (not arrested))*
- **Despatch (radio):** Stern, have you gone BUGGO? *(when Bomb engineer is killed (not arrested))*
- **Despatch (radio):** You arrested one of the engineers! You ASSHOLE! *(when Bomb engineer is arrested)*
- **Despatch (radio):** Is that funny? Handcuffing the engineers? Really? *(when Bomb engineer is arrested)*
- **Despatch (radio):** You arrested one of the engineers! *(when Bomb engineer is arrested)*
- **Despatch (radio):** 2nd Bomb Disposal Unit is on the scene - get that Car Bomb and get it back to them ASAP! *(at the start)*

*Conversation (once both: on entering an area; and #187):*

> **The Sarge:** Well? Come up with anything?  
> **Darci:** I found this old Video tape...It says "UCNN News" on it...  
> **The Sarge:** UCNN? What the hell would the Wildcats be doin' with a tape from UCNN??  
> **Darci:** I think I might know - it could be linked with Gordansky's kidnapping.  
> **The Sarge:** Okay Stern - I'll pass it along to forensics.  


*Conversation (once both: on entering an area; and #188):*

> **The Sarge:** You find anything?  
> **Darci:** Naah, place was full of dust an' old scrap metal.  
> **The Sarge:** Maybe you didn't look hard enough.. still, forget it. We got those two Bombers bang to rights - that's a BIG collar, Stern!  
> **Darci:** Sarge, those guys ain't terrorists - they're just an out-of town Wildcat gang. Experts brought in to do a job.  
> **The Sarge:** What are ya NUTS Stern? The Wildcats couldn't of pulled this off. These terrorists just DRESSED like Wildcats to throw us off.  
> **Darci:** Throw us off? I think they WANTED us to know it was them. Question is, what makes them so sure of themselves that they'd do this...?  

- **Conspiracy theorist:** The cops said there's some kinda bomb scare - but we know the truth! They're conducting SECRET TESTS WITH ALIENS, and cut off the whole area under some PHONEY EXCUSE! Bombers my ass! *(on talking to Conspiracy theorist)*
- **Citizen:** Hell if I know what's goin' on - you're the cop! *(on talking to Citizen)*
- **Citizen:** Whassup, Officer? Killed anyone lately? *(on talking to Citizen)*
- **Citizen:** Is there REALLY a bomb out there? *(on talking to Citizen)*
- **Conspiracy theorist:** They say there's a car bomb? Are you sure it's not a SECRET GOVERNMENT TEST gone WRONG?? HMM? *(on talking to Conspiracy theorist)*
- **Citizen:** Word is, the Wildcats planned all of this! What the hell are they - a gang or a cult or terrorists or WHAT? *(on talking to Citizen)*
- **Citizen:** Bombers! Crazies! What's the world comin' to? *(on talking to Citizen)*
- **Puzzled citizen:** A car bomb? Who the hell would want to bomb a CAR? *(on talking to Puzzled citizen)*
- **Doom-sayer:** This bomb scare don't bode well. No siree, don't bode well at ALL! There's a DOOM on this place, I tell ya... *(on talking to Doom-sayer)*
- **Bane-watcher:** I tell ya, that BANE guy is behind them Wildcats. I seen him around this area once... looked real damn mean, like an' old, ugly bulldog! *(on talking to Bane-watcher)*
- **Cop:** Hi Rookie. Stern ain't it? What you hear about those crazies, the Wildcats? *(on talking to Cop)*
- **Bomb engineer:** Butt out, Rookie! *(on talking to Bomb engineer)*
- **Bomb engineer:** Hey lady, c'mon! *(on talking to Bomb engineer)*
- **Bomb engineer:** I get nervous times like this.. is it the red or blue wire? SHIT! *(on talking to Bomb engineer)*
- **Despatch (radio):** The bomb squad are on the case! *(when the sedan #6 gets near)*
- **Despatch (radio):** Stern, we needed those Terrorists ALIVE! DUMBASS! *(when Bomber is killed (not arrested))*
- **Despatch (radio):** You killed him! Well, there goes a great bust down the drain - along with your CAREER, Stern! *(when Bomber is killed (not arrested))*
- **Despatch (radio):** Aw Jeez - we just got another coded call. A second Car Bomb's been ditched outside Harbours department store. You've got to get it back! *(4s later (earlier events))*
- **Despatch (radio):** What the hell are you doing arresting the engineers? What are you, a MORON? *(when Bomb engineer is arrested)*
- **Despatch (radio):** You arrested one of the engineers! Is this supposed to be FUNNY, Stern? *(when Bomb engineer is arrested)*
- **Despatch (radio):** Arresting the engineers? Is that some sort of JOKE? *(when Bomb engineer is arrested)*
- **Despatch (radio):** You killed one of the engineers! Shit  Stern! *(when Bomb engineer is killed (not arrested))*
- **Despatch (radio):** STERN! You SHOT the engineer! *(when Bomb engineer is killed (not arrested))*
- **Despatch (radio):** What the - Stern, you just shot the ENGINEER! *(when Bomb engineer is killed (not arrested))*
- **Despatch (radio):** The car's blown! *(on a 90s countdown (earlier events))*
- **Despatch (radio):** The car's blown! *(on a 90s countdown (earlier events))*
- **Despatch (radio):** We just got the Terrorist scum spotted heading towards the park! Bring 'em in Stern! *(after Bomber)*
- **Despatch (radio):** You got the bombers, well done! *(once both: when Bomber is arrested; and when Bomber is arrested)*
- **Bomb engineer:** This is gonna be a close call. *(on talking to Bomb engineer)*
- **Bomb engineer:** C'mon Stern. *(on talking to Bomb engineer)*
- **Bomb engineer:** You did well to get that auto back! *(on talking to Bomb engineer)*
- **Despatch (radio):** Bomb squad have it under control. Good work Rookie.. uh, Officer Stern. *(when the sedan #82 gets near)*
- **Despatch (radio):** Those bombers were headin' towards some kinda hideout by the derelict buildings. Search it an' see if you can find anything useful there. *(3s later (after "You got the bombers, well done!"))*

</details>

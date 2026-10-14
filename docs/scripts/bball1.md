# Gatecrasher *(draft; `bball1`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\bball1.ucm`, map `bball1.iam`

**Played as:** Darci

**Story:** An earlier Gatecrasher in which the villain is 'the Judge' ('I guess the Judge needs em pretty soon'), the bouncer sends Darci to 'Big Joe', and there's a club called The Purple Palace.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 6 | Diner host | civilian | civilian |  |
| 31 |  | civilian | civilian |  |
| 32 |  | civilian | civilian |  |
| 33 |  | civilian | civilian |  |
| 34 | Club-goer | civilian | civilian |  |
| 35 |  | civilian | civilian |  |
| 36 |  | civilian | civilian |  |
| 37 |  | civilian | civilian |  |
| 38 |  | civilian | civilian |  |
| 39 |  | civilian | civilian |  |
| 40 |  | civilian | civilian |  |
| 41 |  | civilian | civilian |  |
| 42 |  | civilian | civilian |  |
| 43 |  | civilian | civilian |  |
| 73 |  | Wildcat (rasta) | guard |  |
| 74 |  | Wildcat (rasta) | guard |  |
| 75 |  | Wildcat (rasta) | guard |  |
| 157 | Georgie | workman | cop |  |
| 161 |  | civilian | civilian |  |
| 162 |  | civilian | civilian |  |
| 165 |  | civilian | civilian |  |
| 166 |  | civilian | civilian |  |
| 167 |  | civilian | civilian |  |
| 175 |  | civilian | civilian |  |
| 188 |  | civilian | civilian |  |
| 210 |  | civilian | civilian |  |
| 211 |  | civilian | civilian |  |
| 212 |  | civilian | civilian |  |
| 213 |  | Wildcat (rasta) | civilian |  |
| 214 |  | civilian | civilian |  |
| 215 |  | civilian | civilian |  |
| 216 |  | civilian | civilian |  |
| 217 |  | civilian | civilian |  |
| 218 |  | civilian | civilian |  |
| 219 |  | civilian | civilian |  |
| 220 |  | civilian | civilian |  |
| 221 |  | civilian | civilian |  |
| 222 |  | civilian | civilian |  |
| 223 |  | civilian | civilian |  |
| 224 |  | civilian | civilian |  |
| 225 |  | civilian | civilian |  |
| 226 |  | civilian | civilian |  |
| 227 |  | civilian | civilian |  |
| 228 |  | civilian | civilian |  |
| 230 |  | civilian | civilian |  |
| 258 |  | Wildcat (rasta) | bully |  |
| 278 | Officer Mills | cop (pistol) | cop |  |
| 294 |  | Wildcat (grey) | guard |  |
| 295 |  | Wildcat (grey) | guard |  |
| 297 | Hostage workman | workman | civilian | stands still |
| 298 | Wildcat guard | Wildcat (red) | guard |  |

Others: 15 Wildcat (rasta, pistol) (guard); 12 civilian (driver); 8 Wildcat (rasta) (guard); 6 Wildcat (grey, AK-47) (guard); 5 civilian (civilian); 1 workman (cop); 1 workman (civilian).

</details>

**Objects:**

- Items: barrels x7, health pack x7, shotgun ammo x2, pistol, pistol ammo
- Vehicles: car (unlocked) x10, van (unlocked) x5, van (locked) x2, taxi (unlocked)
- Street furniture: traffic cone x29

**Badges:** 10

**Beacons (places and people the player is sent to):** "Gates to rear access.", "Meeting site.", "Main Gates.", "Mexican Restaurant.", "The Purple Palace.", "Gunman."

**How it ends:**

- **Lost** after "You killed them all, thats no use!".
- **Won** after the conversation "Your comin with me pal, and you can tell...".

<details><summary>Script</summary>


*Conversation (on entering an area):*

> **Wildcat (rasta) #74:** Okay you guys, we need to ship those guns we got stored soon.  
> **Wildcat (rasta) #75:** I guess the Judge needs em pretty soon then?  
> **Wildcat (rasta) #74:** Yeah, he's plannin a few street parties soon if you know what I mean?  


*Conversation (when the player gets near):*

> **Georgie:** Sorry honey, this is wild Bills patch and you ain't comin in unless he says so!  
> **Darci:** How do I find this wild Bill?  
> **Georgie:** Talk to big joe, hes in the club.  

- **Civilian #39:** The Union Black Cats aren't doing so good these days. *(on talking to Civilian #39)*
- **Civilian #167:** I'm going to see lusty Anna. *(on talking to Civilian #167)*
- **Civilian #166:** Shes a saucy one. *(on talking to Civilian #166)*
- **Civilian #165:** Shes a naughty one. *(on talking to Civilian #165)*
- **Civilian #162:** I really do like the girls in here. *(on talking to Civilian #162)*
- **Civilian #161:** Can't wait for this place to open. *(on talking to Civilian #161)*
- **Civilian #175:** In you go lady! *(on talking to Civilian #175)*
- **Civilian #40:** This city is full of scum. *(on talking to Civilian #40)*
- **Civilian #41:** I love baseball, but theres been no games here for weeks. *(on talking to Civilian #41)*
- **Civilian #33:** My pal razor is dead angry with those gangsters. *(on talking to Civilian #33)*
- **Civilian #42:** Baseball ain't my game. *(on talking to Civilian #42)*
- **Civilian #31:** Shucks...I hate this town! *(on talking to Civilian #31)*
- **Civilian #32:** My names Pat, and I can't stand snakes. *(on talking to Civilian #32)*
- **Club-goer:** All recent baseball matches have been fixed I reckon. *(on talking to Club-goer)*
- **Civilian #35:** I work around here, but I wish I didn't. *(on talking to Civilian #35)*
- **Civilian #36:** Hi officer. *(on talking to Civilian #36)*
- **Civilian #37:** Seems this place gets more chaotic every day. *(on talking to Civilian #37)*
- **Civilian #38:** These streets disgust me. *(on talking to Civilian #38)*
- **Civilian #43:** Im off home. *(on talking to Civilian #43)*
- **Diner host:** Hey Gringo..you here on duty or for a meal? *(on talking to Diner host)*
- **Civilian #230:** You have not bought anything. *(on talking to Civilian #230)*
- **Civilian #226:** Hey, you wanna beer? *(on talking to Civilian #226)*
- **Civilian #225:** Hello there, you pretty thing! *(on talking to Civilian #225)*
- **Civilian #221:** Hey look at me...I'm gettin down! *(on talking to Civilian #221)*
- **Civilian #222:** Me mates busy dancin! You wanna drink? *(on talking to Civilian #222)*
- **Civilian #224:** Did it hurt when you fell from Heaven! *(on talking to Civilian #224)*
- **Civilian #223:** Yo babe, dance for me.. *(on talking to Civilian #223)*
- **Civilian #219:** What do you want? *(on talking to Civilian #219)*
- **Civilian #220:** Drink?? *(on talking to Civilian #220)*
- **Civilian #216:** Great tune this! *(on talking to Civilian #216)*
- **Civilian #218:** These girls are great! *(on talking to Civilian #218)*
- **Civilian #215:** Well hello there young lassy! *(on talking to Civilian #215)*
- **Civilian #214:** What a fool! *(on talking to Civilian #214)*
- **Civilian #217:** This club sucks! *(on talking to Civilian #217)*
- **Civilian #227:** Oooohh...baby..baby...laaa! *(on talking to Civilian #227)*
- **Civilian #228:** Baby...I laaarv yoooou.....baby..ooh! *(on talking to Civilian #228)*
- **Civilian #210:** Cha cha cha....! *(on talking to Civilian #210)*
- **Civilian #211:** Do that funky thang ...chick! *(on talking to Civilian #211)*
- **Civilian #212:** Chitta boom cha!! *(on talking to Civilian #212)*

*Conversation (on talking to Officer Mills):*

> **Officer Mills:** Theres something suspect going on in that park, but the chief I have to stay on traffic duty.  
> **Darci:** I don't care what your doing, I'm gonna find out exactly whats going on.  

- **Despatch (radio):** Get into the Union city baseball ground, and be sure to apprenhend at least one of the gang superiors. *(at the start)*

*Conversation (on talking to Wildcat (rasta) #213):*

> **Darci:** Hey you, where can I find Wild Bill?  
> **Wildcat (rasta) #213:** Why do you wanna know?  
> **Darci:** If you don't tell me I'll arrest your sorry ass.  
> **Wildcat (rasta) #213:** Okay, okay, I'll call him up and get him to meet you at the mexican okay.  
> **Darci:** Okay!  

- **Despatch (radio):** You killed them all, thats no use! *(once both: 1s later (when a count is reached); and when a count is reached)*
- **Despatch (radio):** You'll find Bill at Posheetas down the road from here.. *(after Civilian #188)*

*Conversation (on talking to Civilian #188):*

> **Darci:** You must be the one they call Wild Bill.  
> **Civilian #188:** Yeah, I used to be the baseball team captain up until last month. So waddya want?  
> **Darci:** I need to get into the baseball ground through that back alley.  
> **Civilian #188:** If your after those freaks in there I'll be happy to help...follow me..  


*Conversation (once both: when a count is reached; and 1s later (earlier events)):*

> **Darci:** Your comin with me pal, and you can tell me all about your weapon stores.  

- **Bonus:** 200 points. *(once both: once both: when Wildcat (grey) #294 is down; and when Wildcat guard is down; and once both: once both: earlier events; and earlier events; and when Wildcat (grey) #295 is down)*
- **Despatch (radio):** Ive given the guys the go ahead to let you through. See ya! *(5s later (when Civilian #188 gets near))*

*Conversation (once either: earlier events; or earlier events):*

> **Hostage workman:** What do you want with us? Why can't we go back to work?  
> **Wildcat guard:** No ones gonna get hurt if you just stay here with us, while our boss conducts some business in the park!  

- **Despatch (radio):** Err we have a situation involving and armed man, all units in district please attend! *(0s later (after "Ive given the guys the go ahead to let y..."))*
- **Bonus:** Excellent work. *(when Wildcat (rasta) #258 is down)*

</details>

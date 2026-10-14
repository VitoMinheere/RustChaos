# RTA

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\police1.ucm`, map `disturb1.iam`, district West District

**Played as:** Darci

**Briefing:**

> Okay Stern, I guess you're ready to hit the streets now. Call just came in: Reports of an abandoned car holding up traffic on Zarb Avenue. We need you to get over there pronto and return it to the station for impounding.

**Story:** Darci's first beat as a rookie in West District. Despatch sends her to an abandoned taxi after a crash on Zarb Avenue, where two cops are dealing with the drunk who caused it; she drives the taxi back to the station's vehicle drop. On the way an attempted homicide comes in from the Clunton Projects: the victim's friend points her to Kleetus Avenue, where three witnesses argue about how many shots they heard. A mugging crew works the area, and she must bring at least one mugger in alive; one waits on a rooftop. An old tramp hails her by name: Deeks, her late father's snitch from his cop days, who promises to help her. Side jobs: arrest the extra muggers, a man urinating in the park, and a lecherous civilian; talk to Deeks.

**Characters:**

- **Darci Stern**: rookie, first day on the beat
- **Despatch**
- **Sergeant Bryant** (mentioned): wants the taxi impounded
- **Deeks**: tramp, forty years on Union City's streets, Darci's father's old snitch
- **The witness**: the homicide victim's friend; she mustn't be harmed
- **Crash-scene cops** and **the drunk driver**
- **Three muggers**: Wildcats, one on a rooftop with a gun
- **O'Doole** and other cops at the station; streetwalkers in the cells
- Bystanders arguing over two, three or four shots

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 11 | Mugger (Wildcat, identified) | Wildcat (rasta) | civilian | stands still |
| 24 | Passer-by | civilian | civilian |  |
| 30 | The witness | woman | civilian | stands still |
| 54 | Crash witness | civilian | civilian |  |
| 65 | Witness A | civilian | civilian | stands still |
| 67 | Mugging victim | civilian | civilian | stands still |
| 69 | Passer-by | civilian | civilian |  |
| 77 | Passer-by | civilian | civilian | stands still |
| 79 | Crash witness | civilian | civilian |  |
| 84 | Crash-scene cop | cop | cop |  |
| 85 | Crash-scene cop | cop | cop |  |
| 86 | The drunk driver | civilian | civilian |  |
| 91 | Passer-by | civilian | civilian |  |
| 92 | Passer-by | civilian | civilian |  |
| 103 | Mugger (Wildcat) | Wildcat (red) | guard |  |
| 128 | Desk cop | cop | cop |  |
| 160 | Witness B | civilian | civilian | stands still |
| 166 | Witness C | civilian | civilian |  |
| 194 | Deeks | tramp | civilian | stands still |
| 209 | O'Doole | cop (pistol) | cop |  |
| 210 | Woman in the cells | woman | civilian |  |
| 211 | Woman in the cells | woman | civilian |  |
| 212 | Woman in the cells | woman | civilian |  |
| 217 | Cop | cop (pistol) | cop |  |
| 218 | Cop | cop (pistol) | cop |  |
| 219 | Cop | cop (pistol) | cop |  |
| 221 | Cop | cop (pistol) | cop |  |
| 233 | Rooftop mugger (Wildcat) | Wildcat (grey) | guard |  |
| 238 | Man in the park | civilian | civilian | stands still |
| 239 | Woman in the cells | woman | civilian |  |
| 244 | Passer-by | woman | civilian |  |
| 245 | Workman | workman | civilian |  |
| 251 | Lecher | civilian | civilian | guilty |
| 254 | Architecture fan | civilian | civilian |  |

Others: 3 civilian (driver); 2 cop (driver); 1 civilian (a body (dies at once)).

</details>

**Objects:**

- Vehicles: police car (locked) x4, ambulance (unlocked) x3, police car (unlocked) x2, taxi (unlocked), van (locked)
- Street furniture: bin x12, traffic cone x6

**Badges:** 19. **Side objectives:**

- Arrest the Wildcat on a rooftop near Kleetus Ave (1 badge)
- Arrest the Wildcat in the east (1 badge)
- Arrest the civilian in the north-west (1 badge)
- Talk to the tramp in the south-west (1 badge)

**Beacons (places and people the player is sent to):** "Clunton Projects", "Vehicle Drop", "Mugger", "Car crash.", "District Station"

**How it ends:**

- **Lost** once either: "You killed a vital witness!"; or "You arrested a vital witness. She'll nev...".
- **Lost** when the taxi #71 is wrecked.
- **Lost** after "We NEEDED to impound at least one of tho...".
- **Won** once both: when the taxi #71 gets near; and once either: "Mugger arrested."; or earlier events.
- **Lost** when Deeks is down.
- **Lost** once both: once both: 120s later (after Mugger (Wildcat, identified)); and on entering an area; and once both: when Mugger (Wildcat) is killed (not arrested); and "You terminated one of the muggers.".

<details><summary>Script</summary>

- **Despatch (radio):** Arright Rookie, we got a vehicle impact two blocks down. When you're finished your beat, pick up this car on the way back to the station. *(at the start)*
- **Street sign:** Keating Ave *(on entering an area)*
- **Street sign:** Zarb Drv *(on entering an area)*

*Conversation (when the player gets near):*

> **Darci:** Ma'am, emergency services are on the way - did you see where the attacker went?  
> **The witness:** Yes officer - he.. he ran off towards Kleetus Avenue  
> **Darci:** Thank you  Miss, the ambulance should be here any minute.  

- **Street sign:** Kleetus Ave *(on entering an area)*
- **Despatch (radio):** Also got reports of a mugging crew workin' your patch - be good for citizen support if we brought in at least one of 'em. *(at the start)*

*Conversation (on entering an area):*

> **Darci:** Despatch, I've reached the scene... Vehicles in working order.  
> **Crash-scene cop:** 'Kay Stern -take that heap o' junk back to the compound!  

- **Desk cop:** Stern - Sergeant Bryant says you've gotta collect that vehicle for impounding... *(on talking to Desk cop)*
- **Passer-by:** Hey sexy lady - I can help you out, know what I'm sayin'? *(on talking to Passer-by)*
- **Passer-by:** Hi officer. 'Bye officer. *(on talking to Passer-by)*
- **Crash witness:** The guy was obviously drunk. *(on talking to Crash witness)*
- **Despatch (radio):** Kleetus avenue huh? I'd best get over there! *(after the conversation "Ma'am, emergency services are on the way...")*
- **Crash witness:** Jeez, ya should have seen the guy - came round the corner like a Demon from hell, then BAM! - body parts all over da place! YEEHEEHEH! *(on talking to Crash witness)*
- **The drunk driver:** Urrgh.. I aint *hic* drunk.. oshiffer.. *(on talking to The drunk driver)*
- **Crash-scene cop:** Okay Rookie, this is our bust - we'll deal with him. *(on talking to Crash-scene cop)*
- **Crash-scene cop:** Rookie, you just take that car back to the compound double quick! *(on talking to Crash-scene cop)*
- **Passer-by:** Can't BELIEVE you forgot the ball, asshole!. *(on talking to Passer-by)*
- **Passer-by:** Officer... *(on talking to Passer-by)*
- **Woman in the cells:** I never liked the asshole anyway! *(on talking to Woman in the cells)*
- **Woman in the cells:** Drop dead, pig! *(on talking to Woman in the cells)*
- **Woman in the cells:** Lemme go! LEMME GO, YA BUMS! *(on talking to Woman in the cells)*
- **Woman in the cells:** I ain't done nothin'! I was just.. askin' for a lift from the guy! He's a friend o' mine! *(on talking to Woman in the cells)*
- **O'Doole:** Hey there Stern! Any of the old guys give you a hard time, tell 'em O'Doole will kick their ass, okay? *(on talking to O'Doole)*
- **Cop:** Hiya D'arci! Killed anyone yet? HAW HAW HAW.. (ahem).. good day officer.. *(on talking to Cop)*
- **Cop:** Howdie Stern - ain't you supposed to be at the RTA? *(on talking to Cop)*
- **Cop:** Officer, get out there now! *(on talking to Cop)*
- **Cop:** My shift's nearly over, thank God! *(on talking to Cop)*

*Conversation (on entering an area):*

> **Darci:** HEY! Is this your van? Do you have a......Sir, are you urinating against that tree??  
> **Man in the park:** Its a public Park! Back off pig!  

- **Despatch (radio):** Minor offender arrested. *(when Man in the park is arrested)*
- **Workman:** Theres a couple in the park up to no good! *(on talking to Workman)*
- **Passer-by:** Yo miss. *(on talking to Passer-by)*
- **Architecture fan:** The Brooks Tower is a FINE example of Union City architecture. *(on talking to Architecture fan)*
- **Lecher:** Woah! Nice breasts lady. *(on talking to Lecher)*
- **Despatch (radio):** RTA Vehicle Secured.... *(when the taxi #71 gets near)*
- **Crash-scene cop:** Asshole tried to take off! Can you believe that? *(once both: when The drunk driver is down; and on talking to Crash-scene cop)*

*Conversation (when Deeks is seen by Darci):*

> **Deeks:** Hey Copper! Get over here!  
> **Darci:** Huhn?  

- **Bonus:** Mugger terminated. *(when Mugger (Wildcat) is killed (not arrested))*
- **Despatch (radio):** Mugger arrested. *(when Rooftop mugger (Wildcat) is arrested)*
- **Despatch (radio):** Mugger ass pounded.. not bad for a rookie! *(when Mugger (Wildcat) is arrested)*

*Conversation (after "Hey Copper! Get over here!"):*

> **Deeks:** Stern! D'arci Stern! Well I'll be a monkey's Grandma!  
> **Darci:** I don't believe I know you, old man.. you better fill me in REAL QUICK about how you know me!  
> **Deeks:** Hell, I don't know you either, Stern. But your father, ah.. I knew him.  
> **Darci:** My father huh? You mean you knew him from his cop days?  
> **Deeks:** Damn right. He might of mentioned me to you sometime.. my names Deeks!  
> **Darci:** Deeks? I remember - you were one of his snitches!  
> **Deeks:** Thats right, Girlie! Me an' your Dad was REAL pals. I been on the streets of UC for over 40 years! There ain't much goes on in this shit-hole that I don't know about!  
> **Darci:** Deeks, I just started walking the beat today - I could use your help.  
> **Deeks:** You got it girlie! I owe your old man many favors - I sure ain't gonna let his memory down!  
> **Darci:** Thanks Deeks. I gotta hustle - I'll see you later.  
> **Deeks:** See ya Stern - I'll call you at the station if I got any beef for ya..  


*Conversation (when Darci is seen by Rooftop mugger (Wildcat)):*

> **Darci:** You up there! Hey ASSHOLE!  
> **Rooftop mugger (Wildcat):** What you want Cop Scum - Don't you know youre all gonna die?  
> **Darci:** Shit - he's armed!  

- **Despatch (radio):** You terminated one of the muggers. *(when Rooftop mugger (Wildcat) is killed (not arrested))*
- **Despatch (radio):** Stern - we got a call - Attempted Homicide reported in Clunton Projects. Victims friend was a witness. Suspect is white male, medium height.... *(once either: earlier events; or earlier events)*
- **Despatch (radio):** You arrested a vital witness. She'll never talk now! *(when The witness is arrested)*
- **Despatch (radio):** You killed a vital witness! *(when The witness is killed (not arrested))*
- **Despatch (radio):** You've arrested one of the muggers, well done! *(when Mugger (Wildcat, identified) is arrested)*
- **Despatch (radio):** You killed him! *(when Mugger (Wildcat, identified) is killed (not arrested))*
- **Despatch (radio):** Rookie, we finally got a positive ID on one of the muggers. Check your GPS  for the locale, and get to him before he does any more damage -double quick! *(after Mugger (Wildcat, identified))*
- **Witness B:** Sorry Officer I didn't see nothin', but I heard three shots! *(on talking to Witness B)*
- **Witness A:** Sorry Miss, I didn't see nobody.. *(on talking to Witness A)*
- **Witness C:** I heard some shots, then a guy ran past me with a gun! *(on talking to Witness C)*
- **Despatch (radio):** We NEEDED to impound at least one of those guys! *(once both: "You terminated one of the muggers."; and once both: "You killed him!"; and when Mugger (Wildcat) is killed (not arrested))*
- **Despatch (radio):** Aw Jeez Stern! We LOST the goddamn mugger! *(once both: 120s later (after Mugger (Wildcat, identified)); and on entering an area)*

*Conversation (3s later (after "Sorry Officer I didn't see nothin', but ...")):*

> **Witness A:** Excuse me pal, I distinctly heard TWO shots  
> **Witness B:** TWO shots? I'm sorry but it was THREE shots!  
> **Witness A:** Listen, buddy, it was TWO shots.. what are ya DEAF or somethin'?  


*Conversation (after the conversation "Excuse me pal, I distinctly heard TWO sh..."):*

> **Witness C:** Sorry I couldn't help overhearing you guys, but it was FOUR shots that went off, not two or three.  
> **Witness B:** Oh YEAH? SMART guy, heh?  
> **Witness C:** But I..  
> **Witness B:** How's about I take four SHOTS at your FACE pal, Huh?  

- **Darci:** Jeez, I gotta buncha useless lunatics for witnesses.... *(after the conversation "Sorry I couldn't help overhearing you gu...")*
- **Mugging victim:** Thanks lady, you saved my ass! *(once either: 2s later (once both: earlier events; and earlier events); or once both: on talking to Mugging victim; and earlier events)*

</details>

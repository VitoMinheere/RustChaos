# Southside Offensive

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\mission2.ucm`, map `snocrime.iam`, district Southside Beach

**Played as:** Darci

**Briefing:**

> D'arci, we gotta crack down. As of now, I'm commandeering all officers to the Southside to deal with the soaring crime-rate. Wildcats have flooded the area and the cops there can't handle the influx of muggers, thieves and smugglers all over town. We need that crime rate DROPPED Rookie! Get in there and show 'em the stiff boot of the UCPD!

**Story:** The Chief sends every officer to Southside, where crime is soaring and three cops are already down. Darci, partnered with the joking Officer Green, has to drive the crime rate down: dealers, a stalker, a Wildcat (Sweeney) about to settle a score with Fish, an arms handover to Mr Shultz, and Griz executing Walker for talking to Gordansky at Mable Greer's diner. A body turns up with chunks of flesh missing. Roper finally explains himself: the Wildcats aren't the root of it; they're controlled by an outside, far more potent force, and he'll help destroy them. Green and Roper must survive.

**Characters:**

- **Darci Stern**
- **Officer Green**: partner, too fond of jokes
- **Roper**
- **Sweeney** and **Fish**
- **Griz** and **Walker**: Walker killed for talking to Gordansky
- **Mr Shultz**: buyer at an arms deal
- Dealers, a stalked woman, a sceptical cop ('just street scum'), a woman hoping Banes cleans up the subway

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 44 | Waiting man | civilian | civilian |  |
| 65 |  | Wildcat (grey) | assassin |  |
| 67 |  | Wildcat (grey) | assassin |  |
| 146 | Tip-off citizen | civilian | civilian |  |
| 159 | Frightened citizen | civilian | civilian |  |
| 166 | Gas station owner | civilian | civilian |  |
| 167 | Citizen | civilian | civilian |  |
| 212 | Workman | workman | civilian |  |
| 215 | Commuter | civilian | civilian |  |
| 216 | Gun fan | civilian | civilian |  |
| 217 | Bored citizen | civilian | civilian |  |
| 218 | Bane voter | woman | civilian |  |
| 228 | Cop at the body | cop (pistol) | cop |  |
| 237 | Sceptical cop | cop (pistol) | cop |  |
| 238 | Cop | cop (pistol) | cop |  |
| 245 |  | Wildcat (rasta) | guard |  |
| 246 |  | Wildcat (rasta) | guard |  |
| 252 | Dealer | Wildcat (grey) | guard |  |
| 253 | Dealer | Wildcat (grey) | guard |  |
| 254 |  | civilian | civilian |  |
| 255 |  | civilian | civilian |  |
| 266 | Stalked woman | woman | civilian |  |
| 290 | Sweeney | Wildcat (grey) | civilian | stands still |
| 291 | Fish | civilian | civilian |  |
| 309 |  | civilian | civilian |  |
| 310 |  | civilian | civilian |  |
| 313 | Wildcat smuggler | Wildcat (red) | civilian | stands still |
| 314 | Mr Shultz | civilian | civilian | stands still |
| 348 |  | workman | civilian |  |
| 360 | Officer Green | cop | cop |  |
| 362 |  | civilian | civilian |  |
| 365 |  | Wildcat (rasta) | guard |  |
| 366 |  | Wildcat (rasta) | guard |  |
| 372 | Roper | Roper | civilian | stands still |
| 389 | Walker | civilian | civilian | stands still |
| 390 | Griz | civilian | civilian | stands still |
| 413 |  | Wildcat (rasta) | stands about |  |
| 422 |  | Wildcat (rasta) | guard |  |
| 439 |  | Wildcat (red) | civilian | stands still |

Others: 20 Wildcat (grey) (guard); 7 Wildcat (red) (guard); 3 Wildcat (red) (assassin); 3 civilian (driver); 3 Wildcat (grey) (a body (dies at once)); 3 cop (a body (dies at once)); 2 Wildcat (rasta) (guard); 2 Wildcat (grey) (assassin); 2 Wildcat (red) (a body (dies at once)); 1 Wildcat (rasta) (assassin); 1 civilian (cop driver); 1 civilian (a body (dies at once)); 1 Wildcat (rasta) (a body (dies at once)).

</details>

**Objects:**

- Items: barrels x85, mine x4, health pack x3, pistol ammo x3, AK-47 x2, knife x2, shotgun ammo, shotgun, pistol
- Vehicles: van (locked) x4, ambulance (unlocked) x3, taxi (unlocked), police car (locked)
- Street furniture: bin x30
- Set pieces: 2 searchable props

**Badges:** 17. **Side objectives:**

- Take down the Wildcat in the east (1 badge)
- Take down the Wildcat in the south (1 badge)
- Take down the civilian in the south (1 badge)
- Take down the Wildcat on a rooftop in the west (1 badge)
- Talk to the cop in the west (1 badge)

**Beacons (places and people the player is sent to):** "Police Building.", "Officer Green", "Roper."

**How it ends:**

- **Lost** when Officer Green is arrested.
- **Lost** after "SHIT! Roper's dead!".
- **Lost** after "Officer Green is dead! You messed up!".
- **Won** when the crime rate falls.
- **Lost** when the crime rate rises.

<details><summary>Script</summary>

- **Despatch (radio):** Police building secure. *(when the whole group is down)*
- **Despatch (radio):** Jeez, looks like someone wasn't well liked.. *(on entering an area)*

*Conversation (when the player gets near):*

> **Stalked woman:** Officer, there's a MAN following me - I think he's a stalker!  
> **Darci:** Are you sure he's not just a, uh, customer, miss?  
> **Stalked woman:** Hell no! The guy's a nut! He's.. oh no - there he is!  

- **Despatch (radio):** Stern this area of Southside UC is CRAWLIN' with Wildcats - they seem bent on taking over this section. We got THREE officers down already.. I want YOU an' Green to get on this! *(at the start)*
- **Waiting man:** I'm jus' waitin' for my boys. Got a problem with that? *(on talking to Waiting man)*
- **Frightened citizen:** Officer, the crime around here has gone off the scales. Folks are acting CRAZY these last few weeks...! What the hell is wrong with everybody? *(on talking to Frightened citizen)*
- **Gas station owner:** Some idiot rammed the pumps! Damn vandals! *(on talking to Gas station owner)*
- **Citizen:** Damn UC Fire dept. are useless! We called 'em and they say they can't come out here because every time they do, they get attacked by gangs!! *(on talking to Citizen)*
- **Workman:** Hey, I'm just takin a break lady. Give me some peace, huh? *(on talking to Workman)*
- **Bored citizen:** YAWN! *(on talking to Bored citizen)*
- **Bane voter:** Damn trains are ALWAYS late. I hope Banes' keeps his electoral promise an' cleans up the subway system. *(on talking to Bane voter)*
- **Gun fan:** Hey - you got a gun? Can I see it? *(on talking to Gun fan)*
- **Commuter:** Damn trains are never on time. *(on talking to Commuter)*

*Conversation (on talking to Sceptical cop):*

> **Darci:** Take these assholes to the cells - and make sure and grill them about their little cult!  
> **Sceptical cop:** Cult? What are you, crazy? These guys are just street scum, Stern, I think youre being a LITTLE over dramatic here...!  
> **Darci:** Oh yeah? We'll see....  


*Conversation (on talking to Cop):*

> **Cop:** Officer Byrne - were you talking to the suspect earlier today?  
> **Darci:** Uh, yeah Stern, he was helpin' me out with a case, uh, as an informant y'know...  
> **Cop:** Thats all you got to say, huh?  

- **Despatch (radio):** URBAN CHAOS!!!! *(after #448 (end game lose))*
- **Civilian #255:** I ain't doin' nothin'! *(on talking to Civilian #255)*
- **Civilian #254:** Back off Cop, we're clean, right? Been straight fer 8 months now...! *(on talking to Civilian #254)*

*Conversation (when Darci is seen by Dealer):*

> **Dealer:** Get BACK! You think I'm gonna let you lock me down again? I'll KILL you, Cop!!  
> **Darci:** That supposed to scare me? I think I'm gonna have to bust your head, boy!  

- **Despatch (radio):** Dealer down. *(when Dealer is down)*
- **Despatch (radio):** Dealer down. *(when Dealer is down)*

*Conversation (when Sweeney is seen by Darci):*

> **Fish:** Lissen Sweeney - I can GET you what you WANT maan, but it's gonna take TIIIIME, man!  
> **Sweeney:** You screwin' me again, Fish? Are you tryin' ta make me look like a schmuck in this neighborhood Fish?  
> **Fish:** No maan, no, I wouldn't mess with you Sweeney, I was -  
> **Sweeney:** It's my turn now, Fish - now I'M gonna screw YOU!  

- **Bonus:** Way to go. *(when Sweeney is down)*
- **Fish:** Aw Jeez Miss - you saved my ass! How can I thank you? *(on talking to Fish)*
- **Civilian #310:** This place gives me the creeps. *(on talking to Civilian #310)*
- **Civilian #309:** I gotta bad feelin' about this place. *(on talking to Civilian #309)*

*Conversation (when Wildcat smuggler gets near):*

> **Mr Shultz:** I told ya man, everythin's in order. Now, I'm guessin' you got a REAL EXPENSIVE briefcase to give me?  
> **Wildcat smuggler:** Well, I've got THIS to give you  

- **Bonus:** Way to go. *(when Wildcat smuggler is down)*
- **Workman #348:** Somethin' wrong? What are you lookin' at? *(on talking to Workman #348)*

*Conversation (when the player gets near):*

> **Officer Green:** Howdie Stern. Chief tells me you're the departments  genuine "Super Rookie"!  
> **Darci:** You tryin' to be funny, Green? How Is it possible to be funny with a face as messed up ugly as yours?  
> **Officer Green:** Shit D'arci, I was only-  
> **Darci:** Shut it. If you can keep up with me, you'll live, if you can't  youll die. Clear?  
> **Officer Green:** O-Okay Stern, I was jus' messin' around! Sheesh...  
> **Darci:** Fooling around will get you killed, Green. Now lets go.  

- **Civilian #362:** You what? *(on talking to Civilian #362)*
- **Despatch (radio):** Way to go. *(when Wildcat (rasta) #366 is down)*
- **Bonus:** Way to go. *(when Wildcat (rasta) #365 is down)*

*Conversation (when Darci is seen by Roper):*

> **Roper:** D'arci!  
> **Darci:** Roper? Where the hell have you been? You said you'd be in contact....  
> **Roper:** Stern, I wanted to keep you out of all this until it was absolutely necessary - and now it is.  
> **Darci:** What the hell do you mean? Stop being so damn cryptic all the time!  
> **Roper:** It's the Wildcats!  
> **Darci:** What about the Wildcats?  
> **Roper:** D'arci, the Wildcats ain't the root cause of the troubles in Union City..  
> **Darci:** What? Roper, the Wildcats are behind EVERYTHING in Union City! They've killed countless people, caused untold damage to-  
> **Roper:** D'arci, listen first, question after. The Wildcats are being controlled by an outside influence, a far more  potent force than this gaggle of street scum..  
> **Darci:** What? But if-  
> **Roper:** Question later, Stern! For now, know that I will help you, any way I can, in destroying them.. starting now!  

- **Darci:** SHIT! Roper's dead! *(when Roper is down)*
- **Despatch (radio):** Officer Green is dead! You messed up! *(when Officer Green is killed (not arrested))*

*Conversation (when Griz is seen by Darci):*

> **Griz:** You shouldn't of told that reporter Gordansky, Walker...  
> **Walker:** No! I never spoke to her - honest Griz!  
> **Griz:** Walker, my boy SAW you talking to her in Mable Greers diner.  
> **Walker:** Mable Gree.....oh, yeah, now I remember - uh, she was just asking directions to the ladies room!  
> **Griz:** You let me down walker..  
> **Walker:** No! Griz! Don't!  

- **Despatch (radio):** Way to go. *(when Griz is down)*
- **Tip-off citizen:** I seen some dealer-lookout onna roof behind me officer! *(on talking to Tip-off citizen)*
- **Bonus:** Lookout eliminated. *(when Wildcat (rasta) #413 is down)*
- **Despatch (radio):** NO SWEENEY! NOOO! *(after the conversation "Lissen Sweeney - I can GET you what you ...")*
- **Wildcat smuggler:** I hope you're carrying our cargo, Mr. Shultz? *(when Wildcat smuggler is seen by Darci)*
- **Bonus:** Way to go. *(once both: when Wildcat (rasta) #246 is down; and when Wildcat (rasta) #245 is down)*

*Conversation (once both: when Cop at the body gets near; and "Jeez, looks like someone wasn't well lik..."):*

> **Cop at the body:** Aw Mother of.. just LOOK at THAT! The guy has CHUNKS of flesh missing!  
> **Darci:** Beats me....looks like we got a CRAZY on our hands!  

- **Despatch (radio):** Stern! We got a report of a possible drug deal goin down - get on it! *(when Dealer is seen by Darci)*
- **Bonus:** Arrested. *(when Wildcat (red) #439 is arrested)*
- **Despatch (radio):** Shit, you ever hear of due process? *(when Wildcat (red) #439 is killed (not arrested))*
- **Despatch (radio):** Suspect body reported. *(after the conversation "Aw Mother of.. just LOOK at THAT! The gu...")*

</details>

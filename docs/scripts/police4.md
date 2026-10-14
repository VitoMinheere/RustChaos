# Trouble in the Park

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\police4.ucm`, map `disturb1.iam`, district West District

**Played as:** Darci

**Briefing:**

> D'arci - you got to get your ass over to Bushy Park - there's been a whole bunch of gang-bangers loitering with intent. Residents are getting nervous, they think things are going to explode at any minute.

**Story:** Gang-bangers are loitering in Bushy Park and the residents, and reporter Gordansky, are complaining. Darci clears the Wildcats out; the punks talk about 'sinners', 'the fallen', and a CHANGE coming, and two of them arrange a meet at the Ballpark 'tonight'. A mystery man who knew her father introduces himself as Roper and tells her what's going on: Mack Banes, the mayoral candidate, has taken the Wildcats under his wing, and three of their leaders meet tonight at the UC Baseball park. Bring one in and he'll talk. Roper must survive.

**Characters:**

- **Darci Stern**
- **Roper**: mysterious ally, knew Darci's father
- **Mack Banes** (mentioned): mayoral candidate backing the Wildcats
- **The Chief** and **Gordansky** (mentioned)
- Wildcats arranging the meet at the Ballpark
- Two friends fighting over a forgotten ball, a loiterer, residents talking about 'wolves among us'

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 15 | Park regular | civilian | civilian |  |
| 16 | Prophet of wolves | civilian | civilian |  |
| 24 | Loiterer | civilian | civilian |  |
| 25 | Park regular | civilian | civilian |  |
| 37 |  | Wildcat (rasta) | assassin |  |
| 39 |  | Wildcat (grey) | assassin |  |
| 41 | Worried resident | civilian | civilian |  |
| 45 | Wildcat bully | Wildcat (rasta) | bully |  |
| 54 | Park Wildcat | Wildcat (grey) | gang member |  |
| 55 | Nervous resident | civilian | civilian |  |
| 59 |  | Wildcat (red) | guard |  |
| 69 | Loiterer | civilian | civilian |  |
| 83 |  | Wildcat (red) | guard |  |
| 91 | Ball player | civilian | civilian |  |
| 92 | Ball player | civilian | civilian |  |
| 137 |  | Wildcat (rasta) | gang member |  |
| 138 |  | Wildcat (red) | gang member |  |
| 143 |  | Wildcat (grey) | assassin |  |
| 145 | Wildcat ambusher | Wildcat (rasta) | assassin |  |
| 147 |  | Wildcat (rasta) | assassin |  |
| 149 |  | Wildcat (red) | assassin |  |
| 158 | Roper | Roper | cop |  |
| 183 | Wildcat (the meet) | Wildcat (red) | guard |  |
| 184 | Wildcat (the meet) | Wildcat (rasta) | guard |  |

Others: 6 Wildcat (red) (guard); 4 Wildcat (grey) (guard); 4 cop (driver); 2 cop (pistol) (stands about); 2 Wildcat (grey) (assassin); 1 Wildcat (rasta) (assassin); 1 cop (pistol) (cop); 1 Wildcat (red) (stands about); 1 Wildcat (grey) (stands about); 1 Wildcat (rasta) (guard); 1 civilian (driver).

</details>

**Objects:**

- Items: barrels, knife, health pack, baseball bat
- Vehicles: police car (unlocked) x7, car (unlocked) x2, ambulance (unlocked)
- Street furniture: bin x13

**Badges:** 13. **Side objectives:**

- Arrest the Wildcat in the north (1 badge)
- Side objective (2 badges)
- Side objective (1 badge)
- Arrest the civilian in the middle of town (1 badge)
- Side objective (1 badge)
- Arrest the Wildcat in the east (1 badge)

**Beacons (places and people the player is sent to):** "Disturbance", "District Station", "Roper"

**How it ends:**

- **Lost** when Roper is down.
- **Won** 6s later (after "A meet tonight in the Baseball park? I t...").

<details><summary>Script</summary>

- **Park Wildcat:** Stay outta here, Law dog! Get the Hell out o' this City! There's a CHANGE comin'! HAH HAHH! *(on entering an area)*
- **Despatch (radio):** The Wildcats have been pissin' in our own backyard Stern, an' the residents have been voicing their concern to Gordansky an' the press! *(at the start)*
- **Despatch (radio):** Damn Wildcats are starting major trouble! Chief says we gotta clear 'em out - so get your ass to the Park...! *(at the start)*
- **Bonus:** Got the troublemaker. *(when Ball player is arrested)*
- **Bonus:** Asshole arrested.... *(when Ball player is arrested)*
- **Prophet of wolves:** Sister, there are wolves among us....we must ALL be vigilant! *(on talking to Prophet of wolves)*

*Conversation (on talking to Loiterer):*

> **Loiterer:** Wassup, cop?  
> **Darci:** You been hangin' around here all night - any particular reason?  
> **Loiterer:** My reasons are my own - you got a problem with that?  
> **Darci:** You seen any gang bangers around? Call themselves Wildcats?  
> **Loiterer:** I seen a few o' them crazies. I tell ya, they're all on crack! They don' make any SENSE when they talk!  

- **Worried resident:** There's some gangbangers in the park, Officer! They were callin everyone "sinners" and "the fallen" or some bull..! *(on talking to Worried resident)*
- **Nervous resident:** I WAS gonna cut through the park, but now... *(on talking to Nervous resident)*
- **Park regular:** What you want, cop dude? I ain't carrying.. *(on talking to Park regular)*
- **Park regular:** Don't be sweatin' me.. go hassle some REAL criminals, man! *(on talking to Park regular)*
- **Ball player:** Can't believe this Jerk forgot the ball. *(on talking to Ball player)*
- **Ball player:** Officer.. *(on talking to Ball player)*
- **Loiterer:** I get nervous when there's this much trouble. *(on talking to Loiterer)*

*Conversation (at the start):*

> **Wildcat (the meet):** The Ballpark, tonight - and bring your pistol!  
> **Wildcat (the meet):** Gottit! Maan, I can feel what the Boss said was true...everythin's gonna change!  
> **Wildcat (the meet):** You said it - startin' TONIGHT, at the Park.. !  


*Conversation (after "Can't believe this Jerk forgot the ball."):*

> **Ball player:** Jerk? Me?  
> **Ball player:** Yeah you - I drove us 4 miles across town in the raging heat - an' we finally get here an' YOU DON'T GOT THE BALL!!  
> **Ball player:** Ahh forget you - I'm outta here - an' I'm takin' my ball with me!  
> **Ball player:** Oh YEAH?  

- **Despatch (radio):** Good work Rookie. *(once both: when Wildcat (the meet) is arrested; and when Wildcat (the meet) is arrested)*
- **Darci:** Who the hell's that? *(2s later (earlier events))*
- **Despatch (radio):** Watch your back D'arci - you got trouble! *(after Wildcat ambusher)*

*Conversation (once both: once both: earlier events; and earlier events; and when Roper is seen by Darci):*

> **Darci:** Arright spill it mystery man - what's your game?  
> **Roper:** Hmm. Just as blunt as your father I see.  
> **Darci:** What? My FATHER? Waait a minute - do I know you?  
> **Roper:** You may know OF me, but we've never met. Call me Roper  
> **Darci:** "Roper" eh? So how come you know so much about ME "Roper"?  
> **Roper:** D'arci, there is so much you need to know - but now is not the time. So listen up...  
> **Darci:** Shoot, mystery man.  
> **Roper:** Mack Banes, the Mayoral candidate, is plannin' something BIG for Union City after he gets elected. He took a small-time gang under his wing...  
> **Darci:** Waitaminnit....you mean the Wildcats?  
> **Roper:** Yeah - how else d'you think they got so powerful so quickly?  
> **Darci:** But..  
> **Roper:** NO BUTS! There are three leaders meeting tonight at the UC Baseball park..  
> **Darci:** So?  
> **Roper:** SO if you bring one of 'em in, grill him for a while.. he may tell you what you need to begin your journey. Now, I must go.  
> **Darci:** Wait - Roper!  
> **Roper:** Later, D'arci Stern.. later!  

- **Darci:** A meet tonight in the Baseball park? I think I'll go along an' sniff around. And I'm gonna find out about you too... "Roper". *(after the conversation "Arright spill it mystery man - what's yo...")*

</details>

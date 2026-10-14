# Assassin

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Gangorder1.ucm`, map `gang1.iam`, district Gangland

**Played as:** Mako

**Briefing:**

> You got a job to do brother! It's a head roller. Meet your contact for target instructions.

**Story:** A bonus mission for Mako, unlocked at the very end. A 'head roller': Radar, the Wildcats' eyes and ears on a rooftop, names the target, Corrola, a local politician giving the Boss trouble, now under police protection. Mako and a Wildcat partner hit the escort and kill Corrola, then get back to the hideout. A Wildcat contact wants 'a little talk' with Mako if he makes it back.

**Characters:**

- **Mako** (player)
- **Radar**: Wildcat lookout
- **Corrola**: local politician, the target
- **The Boss** (mentioned)
- A Wildcat contact, an informant, Mako's partner, Corrola's police escort

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 10 | Wildcat contact | Wildcat (rasta) | civilian |  |
| 11 | Wildcat informant | Wildcat (rasta) | civilian |  |
| 22 | Cop | cop (pistol) | cop |  |
| 31 | Cop | cop (pistol) | cop |  |
| 106 | Wildcat | Wildcat (grey) | civilian |  |
| 108 | Fired man | civilian | civilian |  |
| 110 | Mako's partner | Wildcat (rasta) | civilian |  |
| 113 | Corrola | civilian | civilian |  |
| 177 | Radar | Wildcat (rasta) | civilian | stands still |
| 180 | Cop | cop (pistol) | guard |  |
| 213 | Working girl | woman | civilian |  |
| 215 | Working girl | woman | civilian |  |

Others: 23 cop (guard); 3 cop (cop driver); 3 cop (cop); 3 cop (pistol) (cop driver); 2 cop (assassin); 1 cop (bodyguard).

</details>

**Objects:**

- Items: barrels x17, health pack x6, pistol ammo x4, shotgun x2, AK-47 x2, AK-47 ammo x2, shotgun ammo x2, pistol
- Vehicles: police car (unlocked) x6
- Street furniture: bin x46
- Set pieces: 2 steam traps

**Beacons (places and people the player is sent to):** "Corrola", "Base", "Radar"

**How it ends:**

- **Lost** when Radar is down.
- **Won** 3s later (after "Shit man, you messed up those fools GOOD...").
- **Lost** when Wildcat contact is down.
- **Lost** when Wildcat informant is down.

<details><summary>Script</summary>

- **Despatch (radio):** Mako, we got a BIG hit for you in this side of town. Look for "Radar", he'll fill you in on the chump to be whacked. *(at the start)*

*Conversation (on talking to Wildcat contact):*

> **Wildcat contact:** You've arrived at last, huh Mako? So, we gonna get to see the big, bad Mako in action tonight?  
> **Mako:** Don't mess with me son - just tell me where I can find Corrola...  
> **Wildcat contact:** Talk to my man Radar onna roof - he's our eyes an' ears..  
> **Mako:** Gotcha.  
> **Wildcat contact:** Oh an Mako? You make it back from this hit, you an me is gonna have a little talk..  


*Conversation (on talking to Wildcat informant):*

> **Wildcat informant:** Wassup man. Mako, ain't it?  
> **Mako:** That's right. You got anythin' you can tell me about Corrola?  
> **Wildcat informant:** Just that he's being helped by the cops. I dunno if he paid them off, or he's needed for a trial or what, but they got his ass covered.  
> **Mako:** Cops.. Sheeit! I mess up cops by the dozen.  
> **Wildcat informant:** Yeah? Well you better - you fail on THIS hit an us Wildcats are gonna hunt you down man.  
> **Mako:** Hmmnnn. You don't want me on your case boy. Step off.  

- **Fired man:** YEAH! I just got fired. Maaan, there ain't nothin' better than bein' a jobless bum! *(on talking to Fired man)*

*Conversation (on talking to Mako's partner):*

> **Mako's partner:** Hey there Mako! I heard o' you - you supposed to be some badass, right?  
> **Mako:** Seems my rep gets around.. so tell me, you the man that's supposed to help me off this guy?  
> **Mako's partner:** Yeah man - I'm ready.  


*Conversation (once either: on entering an area; or on entering an area):*

> **Cop:** Weeell, if it ain't a little Wildcat, all run away from home. What's a schmuck like you doin' outta your patch?  
> **Wildcat:** Get outta my way cop..  
> **Cop:** I think you're gonna come with me, an' I'll take you down this alley an' kick your ass!  
> **Wildcat:** I think not, blue boy. I'm comin' through YOU to get to where I want to go. So you best step aside.  


*Conversation (on talking to Radar):*

> **Mako:** Arright guy, what's the subject?  
> **Radar:** Hey there Mako. I'm glad I finally met your famous ass. The boss is always tellin'...  
> **Mako:** Get to the point Radar. The target?  
> **Radar:** Jeez, always the professional, huh?  
> **Mako:** WHO?  
> **Radar:** Arright man - it's a local politician geek, been giving' the Boss some trouble.. name of Corolla  
> **Mako:** I KNOW that, man! Shit! Ain't none o' you Wildcats got a damn brain?  
> **Radar:** Uh, no - I mean - YES - I mean - Listen, I just heard that Corolla has been picked up by two cops, so go in easy.  
> **Mako:** Cops. Shee-it!  
> **Radar:** Good luck  
> **Mako:** Right..  
> **Radar:** ..damn tightass..  

- **Cop:** YOU! ONNA FLOOR, NOW!! GET THE HELL DOWN! *(when Mako is seen by Cop)*
- **Working girl:** Hey there, big man, want some fun? *(on talking to Working girl)*
- **Working girl:** I like a man with some.. muscle. *(on talking to Working girl)*
- **Despatch (radio):** You SMOKED CORROLA'S ASS!! Now get back to the hideout! *(when Corrola is down)*
- **Despatch (radio):** Shit man, you messed up those fools GOOD! *(once both: when the player gets near; and "You SMOKED CORROLA'S ASS!! Now get back ...")*

</details>

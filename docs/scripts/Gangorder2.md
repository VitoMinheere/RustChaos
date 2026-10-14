# Rat Catch

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Gangorder2.ucm`, map `gang1.iam`, district Gangland

**Played as:** Mako

**Briefing:**

> We need to getta hold of some dude. Meet up with the boys and one of da guys will let ya know the rest..

**Story:** Mako again. A Wildcat contact tells him to collect McLean and Bish and get the job from Spiv: the Boss wants the son of McIntyre, a friend of the cops, brought in unharmed. The son bolts. Run him down to half health and he surrenders and follows Mako to the drop. Kill him and the Boss's punishment is five more dead cops, which Mako relishes. The district crawls with police, and losing Spiv, Bish, McLean or the escort ends the job.

**Characters:**

- **Mako** (player)
- **Spiv**: Wildcat fixer who passes on the Boss's orders
- **Bish** and **McLean**: Wildcat muscle, McLean recruited at gunpoint
- **McIntyre's son**: the target
- **The Wildcat contact** who meets Mako, and a hanger-on who thought Mako would be bigger
- **McIntyre** (mentioned): a friend of the cops
- Police patrols

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 10 | Wildcat contact | Wildcat (rasta) | bodyguard |  |
| 11 | Wildcat hanger-on | Wildcat (rasta) | bodyguard |  |
| 110 | Bish | Wildcat (grey) | civilian | stands still |
| 113 | McLean | Wildcat (red) | civilian | stands still |
| 120 | Spiv | Wildcat (rasta) | civilian | stands still |
| 122 | McIntyre's son | civilian | civilian |  |
| 135 | Frightened passer-by | civilian | civilian |  |

Others: 33 cop (guard); 6 cop (cop); 6 cop (assassin); 2 cop (cop driver); 1 cop (driver).

</details>

**Objects:**

- Items: barrels x10, health pack x3, shotgun ammo x3, shotgun, pistol ammo, pistol
- Vehicles: police car (unlocked) x3
- Street furniture: bin x34
- Set pieces: 3 steam traps, 1 mist banks

**Beacons (places and people the player is sent to):** "Spiv", "Base", "McLean", "Bish", "Target"

**How it ends:**

- **Lost** when Wildcat contact is down.
- **Won** once both: "WOOAAA!! You the man! The boss kinda lik..."; and "Looks like you blew it, but hey, you an'...".
- **Lost** after "Spiv took a dive!".
- **Lost** after "Mcleans down! The ops OFF!".
- **Lost** after "Bish has screwed it. The ops OFF!".
- **Won** after "You got him! Damn Mako always brings the...".
- **Lost** when Wildcat hanger-on is down.

<details><summary>Script</summary>

- **Despatch (radio):** Mako - We've sent you to this part of town to help the boys grab some guy, a Government asshole's son. Meet with the guys and make that grab! *(at the start)*

*Conversation (on talking to Wildcat contact):*

> **Wildcat contact:** Mako, we brought in someone of your.. talents in order to get a thing done. You do this an' the Boss will be mighty pleased.  
> **Mako:** He better be. So what's the beef?  
> **Wildcat contact:** You've gotta get to Spiv, he'll fill you in. Meantime, pick up McLean and Bish to help you out.  
> **Mako:** Bish and Mclean? I hope they ain't candy asses like most o' you guys. I ain't foolin' around here.  
> **Wildcat contact:** Don' worry man, they'll break all the right skulls.  
> **Mako:** They better. Else I'll come back here an' take it out of ALL your asses.  

- **Wildcat hanger-on:** You really Mako? I thought you'd be bigger.... *(on talking to Wildcat hanger-on)*
- **Despatch (radio):** WOOAAA!! You the man! The boss kinda likes you again! *(when a count is reached)*
- **Despatch (radio):** Mcleans down! The ops OFF! *(when McLean is down)*
- **Despatch (radio):** Bish has screwed it. The ops OFF! *(when Bish is down)*

*Conversation (on talking to Bish):*

> **Bish:** You Mako? WOW! Hey, I'm Bish.. what's goin' down?  
> **Mako:** Bish? Mako. I been told you're a good man in a fight?  
> **Bish:** Sure I am! I be breakin' heads by the..  
> **Mako:** Can it. Follow me.  
> **Bish:** Arright, man. No need to get uppity with Bish.  


*Conversation (on talking to McLean):*

> **McLean:** Hey, you seen some asshole with -  
> **Mako:** I ain't got time. McLean, Boss says you're to come with me!  
> **McLean:** With you? Who the hell you? Why should I -  
> **Mako:** My names Mako - and if you don't come with me now, I'll shoot you through the kneecaps an' leave you to bleed to death right here in the street.  
> **McLean:** But I.. uh.. uhh, okay.  

- **Frightened passer-by:** I- I don't want no trouble now! *(on talking to Frightened passer-by)*
- **Despatch (radio):** Hey you bums - LISSEN UP! You got a new location to go to - check your location an' HAUL ASS! *(once both: after the conversation "Hey, you seen some asshole with -"; and after the conversation "You Mako? WOW! Hey, I'm Bish.. what's go...")*
- **Despatch (radio):** Spiv took a dive! *(when Spiv is down)*

*Conversation (on talking to Spiv):*

> **Mako:** Hey Spiv! Good to see ya again.  
> **Spiv:** Mako! Your ass finally made it, huh? My boys thought you'd bummed out on us.  
> **Mako:** Hell no. So what's the beef?  
> **Spiv:** Guy by the name of McIntyre, a cops friend - Boss wants his son taken.  
> **Mako:** What the hell for?  
> **Spiv:** I don't ask, man! I do what I'm told, like you will!  
> **Mako:** Arright Spiv. I'll get goin', but I ain't hurtin' him!  
> **Spiv:** No need to man, Just deliver him to us an' we'll take care of him for ya!  


*Conversation (on talking to Spiv):*

> **Spiv:** Looks like you blew it, but hey, you an' me know he deserved it!  
> **Mako:** Damn right brother!  
> **Spiv:** Okay, the districts crawlin' with cops now. The boss wants ya to take out at least 5 more of 'em.. he says that's your punishment.  
> **Mako:** PUNISHMENT! HA.. I love that shit!  

- **Despatch (radio):** You got him! Damn Mako always brings the shit! *(when McIntyre's son gets near)*

*Conversation (when McIntyre's son is seen by Mako):*

> **McIntyre's son:** Woooa.. who are you guys?  
> **Mako:** You see this big gun I have? I will shoot you in the teeth if you don't follow me right now..  
> **McIntyre's son:** You crazy man? I - I ain't goin' nowhere with you Wildcat scum! You think I'm stupid?  
> **Mako:** Lissen boy, this is for you're own good: DO AS I SAY!  
> **McIntyre's son:** To hell with you man, you'll have to kill me first!  

- **Despatch (radio):** SHIT Mako - you KILLED the guy you freakin SCREW UP! Get back to base! *(when McIntyre's son is down)*
- **Despatch (radio):** Nice, Mako...that guy was just TOO dumb. *(once both: when the player gets near; and "SHIT Mako - you KILLED the guy you freak...")*

*Conversation (once both: when the player gets near; and after "SHIT Mako - you KILLED the guy you freak..."):*

> **Wildcat contact:** Mako, you asshole! That operation went down like a sack of shit! The boss won't be pleased!  
> **Mako:** Look man, he put up a fight, HE ASKED FOR IT!  
> **Wildcat contact:** Just get your ass back to the plaza and speak with Spiv!  

- **McIntyre's son:** Okay okay! I SURRENDER!! *(3s later (when McIntyre's son is down to half health))*

*Conversation (3s later (after "Okay okay! I SURRENDER!!")):*

> **Mako:** So you gonna come peaceful? Give me any more shit an' your DEAD!  
> **McIntyre's son:** Yeah okay man, whatever you say, I'll come with you!  


*Conversation (once both: "So you gonna come peaceful? Give me any ..."; and when McIntyre's son is seen by Mako):*

> **Mako:** So you gonna come peaceful? Give me any more shit an' youre DEAD!  
> **McIntyre's son:** Yeah okay man, whatever you say, I'll come with you!  


</details>

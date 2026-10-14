# Nitro Car

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Bankbomb1.ucm`, map `gang1.iam`, district Gangland

**Played as:** Mako

**Briefing:**

> Okay bro', we got a car needs parking outside the fed bank. This car ain't jus' hot, but it's ready to blow if you catch my drift.

**Story:** The first mission on the other side of the law: Mako, a Wildcat heavy brought in from outside, is told by the Boss's man to cause CHAOS. He drives a car bomb to the bank courtyard and runs before it blows, then takes a second car bomb (the Boss's explosives expert's work) to a department store guarded by a police bomb squad. Cliffy and Zit, two Wildcats touchy about their nicknames, back him up and take on the cops. Losing either of them, or wrecking a car bomb before it's delivered, fails it.

**Characters:**

- **Mako**: Wildcat enforcer (player)
- **The Boss's voice** on the radio
- **Cliffy** and **Zit**: Wildcat bodyguards
- **Police bomb squad**
- Cops who jump Cliffy and Zit at the end (guilty)

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 3 | Zit | Wildcat (red) | bodyguard | stands still |
| 4 | Cliffy | Wildcat (rasta) | bodyguard | stands still |
| 34 | Bomb squad cop | cop | guard |  |
| 35 | Bomb squad cop | cop | guard |  |
| 36 | Bomb squad cop | cop | guard |  |
| 104 | Cop | cop | guard | guilty |
| 124 | Cop | cop | guard | guilty |

Others: 19 cop (guard); 8 cop (assassin).

</details>

**Objects:**

- Items: barrels x7, health pack x4, knife x2, pistol ammo
- Vehicles: sedan (unlocked), meat wagon (locked), sedan (locked)
- Street furniture: bin x21, traffic cone x6

**Beacons (places and people the player is sent to):** "Bomb drop.", "Car pick up point.", "Bomb drop point 2", "Cop demo squad.", "Carbomb 2", "Cliffy", "Zit"

**How it ends:**

- **Lost** after "Mako you psycho! You got one of the boys...".
- **Won** after the conversation "HAAH HA! Two damn HITS an' several cops ...".
- **Lost** when the sedan #2 is wrecked.
- **Lost** when the sedan #37 is wrecked.
- **Lost** when the sedan #37 is wrecked.
- **Lost** when the sedan #2 is wrecked.

<details><summary>Script</summary>

- **Despatch (radio):** Tonight we're gonna cause CHAOS in UC! We got us a nice bomb.. an' here's your vehicle of DESTRUCTION! hehe heh! *(at the start)*
- **Cliffy:** Hey Mako! I hear we're goin' ta be engaging in some DESTRUCTION tonight! Hehhheeh heh! *(on talking to Cliffy)*
- **Despatch (radio):** Boss wants chaos, we're gonna give it to him. This BANK needs a BOMB delivered to it.... *(when Mako is in the sedan #2)*

*Conversation (on talking to Zit):*

> **Zit:** I'm ready man - we got your back in' case it all goes to hell. For now, best get this car over to the bank!  
> **Mako:** Who're you, Cliffy or Zit?  
> **Zit:** I'm Zit - Cliffy's my man over there. And don't be plannin' on cussing our handles - we get REAL touchy about that.  
> **Mako:** I hear ya. So, the bank it is, huh?  

- **Despatch (radio):** Get our hot rod over to the corner o' this DEPARTMENT STORE. Bail out an' let Nature take it's course! *(at the start)*
- **Despatch (radio):** .. but we just got news of a cop bomb squad movin' in. They're expectin trouble so get the boys to help you take 'em out before ditchin' da plastic. *(at the start)*

*Conversation (on talking to Cliffy):*

> **Mako:** Cliffy - you got my back in case the BLUE BOYS arrive?  
> **Cliffy:** Yeah Mako - pleasure to work with a man o' your CALIBRE!  


*Conversation (on talking to Zit):*

> **Mako:** Zit, we got some cops need squeezin'!  
> **Zit:** Hey maan, I told you I don't like people cussin' my handle, all right?  
> **Mako:** Can it. You wanna tag along for the ride or NOT?  
> **Zit:** Hell Mako - I was only playin with you.  
> **Mako:** IN or OUT?  
> **Zit:** Damn right I'm in! I ain't murderized a cop for MONTHS!  

- **Despatch (radio):** The Bank's across town. Drive our Car Bomb CAREFUL like to the Bank courtyard an' then CLEAR OUT! *(at the start)*
- **Despatch (radio):** Okay Mako, explosives primed. Get your ass outta there! *(once both: when the sedan #2 gets near; and when Mako is in the sedan #2)*
- **Despatch (radio):** Ditch the car Mako! *(when the sedan #2 gets near)*
- **Despatch (radio):** Mako you psycho! You got one of the boys killed! *(once either: when Cliffy is down; or when Zit is down)*
- **Despatch (radio):** That should have made a dent! *(on a 15s countdown (after "Okay Mako, explosives primed. Get your a..."))*
- **Despatch (radio):** Yeehaa! That's one down, one to go.. here's your new car, courtesy of the BOSS an' his EXPLOSIVES expert! *(3s later (earlier events))*
- **Despatch (radio):** Leave it here! *(when the sedan #37 gets near)*
- **Despatch (radio):** Leave the car alone, get da cops first! *(once both: the sedan #37; and on pressing action here)*
- **Despatch (radio):** Okay man, charges set- get the hell OUTTA there! *(once both: "Leave it here!"; and when Mako is in the sedan #37)*
- **Despatch (radio):** Shit Mako - you really do live up to your Rep! Those COPS is TOAST! *(once both: when Bomb squad cop is down; and once both: when Bomb squad cop is down; and when Bomb squad cop is down)*
- **Despatch (radio):** Jeez Mako, you left Cliffy an' Zit to the mercy o' the COPS! Get back there an' brutalize that Cop! *(5s later (earlier events))*

*Conversation (once either: when Cop is down; or when Cop is down):*

> **Zit:** HAAH HA! Two damn HITS an' several cops downed! Man, the Boss will be pleased with this! We'll all be driving LIMO's, Mako!  
> **Mako:** Henh.. yeah.  


</details>

# Cop Killers

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\AWOL1.ucm`, map `gpost3.iam`, district Rio canal

**Played as:** Darci, then Roper, Darci

**Briefing:**

> Damn Wildcats have us against the wall, Officer Stern! They say they've got TWO UCPD officers held, and are claiming they're gonna shoot them if we don't leave them alone. We can't let the uniform down like this Stern! This is a top priority case for a rookie, Stern, but I got faith in you lady - get Johnson and Beck out of there unharmed, or else you'll lose your badge.

**Story:** Roper's warning comes true: the Wildcats have declared war on the UCPD and snatched officers Johnson and Beck, threatening to shoot them unless the police back off. The streets around the Rio canal are sown with military-surplus mines. Roper turns up ('you ain't the only friend I got in the UCPD'): Darci frees Johnson and walks him to the safe zone, then play passes to Roper, who carries the wounded Beck back. Overheard: the Southside Wildcats have 'all KIND o' artillery' cooking for the UCPD.

**Characters:**

- **Darci Stern**, then **Roper**
- **Officer Johnson**: captured cop, proud
- **Officer Beck**: captured cop, wounded
- Wildcat captors, a mocking punk, a tramp who hands out a gift, a civilian held by a Wildcat

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 3 | Officer Beck | cop | lies injured | stands still |
| 5 | Officer Johnson | cop | civilian | guilty, stands still |
| 12 |  | Wildcat (red) | guard |  |
| 88 |  | Wildcat (red) | guard |  |
| 89 |  | Wildcat (grey) | guard |  |
| 90 |  | Wildcat (grey) | guard |  |
| 96 | Roper | Roper | bodyguard | stands still |
| 139 |  | Wildcat (red) | guard |  |
| 140 |  | Wildcat (red) | guard |  |
| 144 |  | Wildcat (rasta) | guard |  |
| 213 |  | Wildcat (red) | guard |  |
| 227 | Mocking Wildcat | Wildcat (red) | stands about | stands still |
| 251 |  | Wildcat (red) | guard |  |
| 280 | Captive civilian | civilian | civilian | stands still |
| 281 | His captor | Wildcat (red) | guard |  |
| 328 | Wildcat gossip | Wildcat (red) | guard |  |
| 329 | Wildcat gossip | Wildcat (grey) | guard |  |
| 389 | Tramp | tramp | civilian | stands still |

Others: 33 Wildcat (grey) (guard); 21 Wildcat (red) (guard); 18 Wildcat (rasta) (guard); 4 Wildcat (red) (Man In Black); 1 Wildcat (red) (assassin); 1 Wildcat (grey) (assassin).

</details>

**Objects:**

- Items: mine x84, barrels x83, health pack x9, pistol ammo x5, AK-47 x3, AK-47 ammo x3, shotgun ammo x3, explosives x2, knife, grenade
- Vehicles: taxi (locked) x6, sedan (locked) x3, meat wagon (locked), police car (locked)
- Street furniture: traffic cone x7
- Set pieces: 2 mist banks

**Badges:** 23. **Side objectives:**

- Talk to the tramp in the north-west (3 badges)

**Beacons (places and people the player is sent to):** "Beck", "Johnson", "Safe Zone."

**How it ends:**

- **Lost** after "Johnson's dead! You blew it BIG TIME, D'...".
- **Won** 4s later (after "You saved them both! Good work Stern!").
- **Lost** after "Shit! Roper's down! I'm on my own now.".

<details><summary>Script</summary>

- **Despatch (radio):** Gate Open *(on pressing action here)*
- **Despatch (radio):** Seems your hunch about Wildcats declaring war on UCPD was right Stern - Bastards nabbed officers Johnson and Beck  an hour ago! *(at the start)*

*Conversation (on entering an area):*

> **Wildcat gossip:** ..our boys over in the SOUTHSIDE got a real niiice dish cookin' for the UCPD..  
> **Wildcat gossip:** I hear they got their hands on all KIND o' artillary down there. What they goin' do with it all? They're gonna bring CHAOS to southside, just you watch!  

- **Despatch (radio):** Johnson's dead! You blew it BIG TIME, D'arci!! *(when Officer Johnson is killed (not arrested))*
- **Despatch (radio):** Roper, go get Beck! *(when Roper is seen by Darci)*
- **Despatch (radio):** You got Beck through Stern.. way to go! *(when Officer Beck gets near)*
- **Despatch (radio):** We got no choice but to get 'em out Stern, and on the QT. Find them an' bring 'em back alive - ANY WAY YOU CAN! *(at the start)*
- **Despatch (radio):** Johnson, you're safe! *(when Officer Johnson gets near)*

*Conversation (at the start):*

> **Darci:** Roper - you turn up at the weirdest times....!  
> **Roper:** I heard  you got a couple of your guys grabbed - thought I'd come lend you a hand..  
> **Darci:** Thanks, cant say I dont need it. But how'd you know the Wildcats took Johnson and Beck?  
> **Roper:** You ain't the only friend I got in the UCPD, D'arci..  
> **Darci:** I hear ya. For now I'll go get Johnson, when I'm done, YOU go get Beck. Sound good?  
> **Roper:** Understood, girl. Now go.  

- **Despatch (radio):** But tread carefully - they got their damn hands on military surplus MINES and have covered the streets!! *(at the start)*
- **Roper:** What you waitin' for? Get in there! *(on talking to Roper)*
- **Darci:** Shit! Roper's down! I'm on my own now. *(when Roper is down)*

*Conversation (when Darci gets near):*

> **Mocking Wildcat:** Hey Blue! How about you try arrestin' ME?  
> **Darci:** What, a two-bit punk like you? Hell no, the guys down the station would all LAUGH at me!  
> **Mocking Wildcat:** Hn? Why you.. think that's funny huh? You wait whore, I'm comin' to FINISH YOUR ASS!  

- **Bonus:** Hostage Safe. *(when His captor is down)*
- **Despatch (radio):** KNOCK AND RUN! *(once both: when the player gets near; and on pressing action here)*
- **Despatch (radio):** You saved them both! Good work Stern! *(once both: "Johnson, you're safe!"; and "You got Beck through Stern.. way to go!")*
- **Darci:** Get him back to the truck! *(once both: when Roper gets near; and when Roper is seen by Darci)*
- **Captive civilian:** Thanks! *(once both: when His captor is down; and on talking to Captive civilian)*
- **Tramp:** Ain't you got better things to do than be knockin' on my door? Here, go play with these.. *(on talking to Tramp)*

*Conversation (once both: once both: when Wildcat (grey) #89 is down; and when Wildcat (grey) #90 is down; and once both: when the player gets near; and once both: earlier events; and earlier events):*

> **Darci:** Johnson, you okay? Those assholes hurt you?..  
> **Officer Johnson:** Hell no Stern, I beat down guys far tougher than them before!  
> **Darci:** Oh, so that's why they kicked your ass an' trussed you up like an amateur rent-a-cop, huh?  
> **Officer Johnson:** Arright Stern - you've had your laugh - save your breath for the journey back!  
> **Darci:** Then move it, Johnson!  

- **Roper:** I'm gonna have to carry this guy back! *(once both: when Roper gets near; and once both: once both: earlier events; and earlier events; and once both: earlier events; and earlier events)*
- **Darci:** Roper, I got 'em secured. You best head on after Beck now. An' go easy, this area of the city is CRAWLIN' with Wildcats an' cops! *(after the conversation "Johnson, you okay? Those assholes hurt y...")*

</details>

# Cop Killers *(draft; `Acarry`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Acarry.ucm`, map `gpost3.iam`

**Played as:** Darci, then Roper, Darci

**Story:** Cop Killers almost as shipped: the carrying test ('A carry').

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 3 | Officer Beck | cop | lies injured | stands still |
| 5 | Officer Johnson | cop | civilian | guilty, stands still |
| 89 |  | Wildcat (grey) | guard |  |
| 90 |  | Wildcat (grey) | guard |  |
| 96 | Roper | Roper | civilian | stands still |
| 139 |  | Wildcat (red) | guard |  |
| 140 |  | Wildcat (red) | guard |  |
| 144 |  | Wildcat (rasta) | guard |  |
| 213 |  | Wildcat (red) | guard |  |
| 227 | Mocking Wildcat | Wildcat (red) | guard |  |
| 251 |  | Wildcat (red) | guard |  |
| 280 | Captive civilian | civilian | civilian | stands still |
| 281 | His captor | Wildcat (red) | guard |  |
| 328 | Wildcat gossip | Wildcat (red) | guard |  |
| 329 | Wildcat gossip | Wildcat (grey) | guard |  |
| 389 | Tramp | tramp | civilian | stands still |

Others: 30 Wildcat (grey) (guard); 19 Wildcat (red) (guard); 16 Wildcat (rasta) (guard); 4 Wildcat (red) (Man In Black); 1 civilian (a body (dies at once)).

</details>

**Objects:**

- Items: mine x84, barrels x83, health pack x6, pistol ammo x5, AK-47 x3, AK-47 ammo x3, explosives x2, knife, shotgun ammo, grenade
- Vehicles: taxi (locked) x6, sedan (locked) x3, meat wagon (locked), police car (locked)
- Street furniture: traffic cone x7
- Set pieces: 2 mist banks

**Badges:** 24. **Side objectives:**

- Talk to the tramp in the north-west (3 badges)

**Beacons (places and people the player is sent to):** "Beck", "Johnson", "Safe Zone."

**How it ends:**

- **Lost** after "Johnson's dead! You blew it BIG TIME, D'...".
- **Won** after "You saved them both! Good work Stern!".
- **Lost** after "Shit! Roper's down! I'm on my own now.".

<details><summary>Script</summary>

- **Despatch (radio):** Gate Open *(on pressing action here)*
- **Despatch (radio):** Seems your hunch about Wildcats declarin' war on UCPD was right Stern - Officers Johnson and Beck were nabbed by a gang an hour ago! We need you to get them back safe! *(at the start)*

*Conversation (on entering an area):*

> **Wildcat gossip:** ..our boys over in the SOUTHSIDE got a real niice dish cookin' for the UCPD..  
> **Wildcat gossip:** I hear they got their hands on all KIND o' artillary down there. What they goin' do with it all? They're gonna bring CHAOS to southside, just you watch!  

- **Despatch (radio):** Johnson's dead! You blew it BIG TIME, D'arci!! *(when Officer Johnson is killed (not arrested))*
- **Despatch (radio):** Roper, go get Beck! *(when Roper is seen by Darci)*
- **Despatch (radio):** You got Beck through Stern - well done! *(when Officer Beck gets near)*
- **Despatch (radio):** We got no choice but to get 'em out Stern, and on the QT. Find 'em an' bring 'em back - ANY WAY YOU CAN! *(at the start)*
- **Despatch (radio):** Johnson, you're safe! *(when Officer Johnson gets near)*

*Conversation (at the start):*

> **Darci:** Roper - you turn up at the wierdest times....!  
> **Roper:** I heard  you got a couple of your guys grabbed - thought I'd come lend you a hand....  
> **Darci:** I ain't saying I don't need help, but how'd  you know the Wildcats got Johnson and Beck?  
> **Roper:** You ain't the only friend I got in the UCPD, D'arci..  
> **Darci:** Okay. For now, I'll go get Johnson. When I'm done, YOU go get Beck, gottit?  
> **Roper:** Understood, girl. Now go.  

- **Despatch (radio):** But tread carefully - they got their hands on military surplus MINES, an' have covered entire areas!! *(at the start)*
- **Roper:** What you waitin' for? Get in there! *(on talking to Roper)*
- **Darci:** Shit! Roper's down! I'm on my own now. *(when Roper is down)*

*Conversation (when Darci is seen by Mocking Wildcat):*

> **Mocking Wildcat:** Hey BITCH! How about you try arrestin' ME?  
> **Darci:** What, a two-bit punk like you? Hell no, the guys down the station would all LAUGH at me!  
> **Mocking Wildcat:** Hn? Why you....think that's funny huh? You wait bitch, I'm comin' to FINISH YOUR ASS!  

- **Bonus:** Hostage Safe. *(when His captor is down)*
- **Despatch (radio):** KNOCK AND RUN! *(once both: when the player gets near; and on pressing action here)*
- **Despatch (radio):** You saved them both! Good work Stern! *(once both: "Johnson, you're safe!"; and "You got Beck through Stern - well done!")*

*Conversation (once both: once both: when Wildcat (grey) #89 is down; and when Wildcat (grey) #90 is down; and when the player gets near):*

> **Darci:** Johnson, you okay? Those assholes hurt you?..  
> **Officer Johnson:** Hell no Stern, I beat down guys far tougher than them before!  
> **Darci:** Oh, so that's why they kicked your ass so hard an' trussed you up like an amateur rent-a-cop, huh? 'Cos they're all wimps, right?  
> **Officer Johnson:** Arright Stern - you've had your laugh - save your breath for the journey back!  
> **Darci:** Then move it, Johnson!  

- **Darci:** Get him back to the truck! *(once both: when Roper gets near; and when Roper is seen by Darci)*
- **Captive civilian:** Thanks! *(once both: when His captor is down; and on talking to Captive civilian)*
- **Tramp:** Ain't you got better things to to than be knockin' on my door? Here, go play with these.. *(on talking to Tramp)*
- **Roper:** I'm gonna have to carry this guy back! *(once both: when Roper gets near; and once both: once both: earlier events; and earlier events; and once both: earlier events; and earlier events)*
- **Darci:** Roper, I got 'em secured. You best head on after Beck now. An' go easy, this area of the city is CRAWLIN' with Wildcats an' cops! *(after the conversation "Johnson, you okay? Those assholes hurt y...")*

</details>

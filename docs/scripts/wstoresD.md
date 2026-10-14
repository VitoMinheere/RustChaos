# Arms Breaker *(late draft; `wstoresD`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\wstoresD.ucm`, map `gpost3.iam`

**Played as:** Darci

**Story:** Arms Breaker almost as shipped.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 8 | Cabbie | civilian | civilian |  |
| 18 | Wildcat gossip | Wildcat (red) | stands about | stands still |
| 19 | Wildcat gossip | Wildcat (grey) | stands about | stands still |
| 21 | Alien hunter | workman | civilian |  |
| 22 | Working girl | woman | civilian |  |
| 65 | Working girl | large woman | civilian |  |
| 67 | Working girl | woman | civilian |  |
| 96 | Pick-up team | Wildcat (rasta) | guard |  |
| 97 | Pick-up team | Wildcat (red) | guard |  |
| 162 | Spinks | Wildcat (grey) | guard | stands still |
| 163 | Wildcat guard | Wildcat (red, shotgun) | guard | stands still |
| 174 | Tip-off citizen | civilian | civilian |  |
| 202 | 'Bloody' Mary Schwartz | large woman | civilian |  |
| 230 | Working girl | woman | civilian |  |
| 232 | Hot dog vendor | civilian | civilian |  |
| 244 |  | Wildcat (rasta) | guard | stands still |
| 245 |  | Wildcat (rasta) | guard | stands still |
| 246 | Assault victim | civilian | civilian | stands still |
| 260 | Cop | cop (pistol) | cop |  |
| 261 | Cop | cop (pistol) | cop |  |
| 262 | Cop | cop (pistol) | cop |  |
| 291 | Roper | Roper | civilian | stands still |
| 341 | Man in the alley | civilian | civilian | stands still |
| 344 |  | Wildcat (rasta) | guard |  |
| 381 |  | civilian | civilian |  |
| 383 | Flasher | workman | civilian |  |
| 384 | Working girl | woman | civilian |  |
| 395 |  | Wildcat (rasta) | guard |  |
| 396 |  | Wildcat (rasta) | guard |  |
| 413 |  | Wildcat (red) | guard | guilty |
| 414 |  | Wildcat (grey) | guard |  |
| 417 | Barmaid | woman | civilian |  |
| 423 |  | civilian | civilian |  |

Others: 6 Wildcat (red, shotgun) (guard); 5 Wildcat (grey) (guard); 4 Wildcat (rasta, pistol) (guard); 4 Wildcat (red) (guard); 3 Wildcat (grey, AK-47) (guard); 2 Wildcat (rasta, pistol) (bully); 1 Wildcat (grey) (gang member); 1 cop (pistol) (driver); 1 civilian (driver); 1 cop (shotgun) (driver); 1 Wildcat (rasta) (guard); 1 civilian (civilian); 1 Wildcat (red, shotgun) (bully); 1 Wildcat (red) (driver).

</details>

**Objects:**

- Items: barrels x35, health pack x8, explosives x6, AK-47 ammo x5, pistol ammo x4, shotgun ammo x3, pistol x2, AK-47
- Vehicles: taxi (locked) x6, police car (unlocked) x2, van (unlocked) x2, van (locked) x2, ambulance (unlocked), van (red key)
- Street furniture: bin x29

**Badges:** 13. **Side objectives:**

- Side objective (1 badge)
- Side objective (1 badge)

**Beacons (places and people the player is sent to):** "Pick up team", "Roper", "Wildcat stores"

**How it ends:**

- **Lost** when Roper is down.
- **Won** once both: once both: "Good work Stern. But don't get TOO cocky..."; and "Supplies sabataged!"; and "Pick up team eliminated.".

<details><summary>Script</summary>

- **Despatch (radio):** Holly Vale *(on entering an area)*
- **Street sign:** Banner Avenue *(on entering an area)*
- **Street sign:** Reagan Plaza *(on entering an area)*
- **Street sign:** Rio Canal Boathouse *(on entering an area)*
- **Bonus:** You got 'em Stern! Well done! *(when the whole group is down)*
- **Despatch (radio):** Good work Stern. But don't get TOO cocky. *(when a prop is damaged)*

*Conversation (when the player gets near):*

> **Darci:** Arright jerk, put it away and get lost. This ain't a public Lavatory!  
> **Man in the alley:** Heeeey - I'm just takin' a leak - or are bodily functions illegal TOO now, huh?  
> **Darci:** BEAT IT creep, before I DOWN you!  

- **Despatch (radio):** Supplies sabataged! *(when a prop is damaged)*

*Conversation (when the player gets near):*

> **Darci:** HEY! ASSHOLE! YOU WANNA HAND THERE?.  
> **Flasher:** Wha...? Uuuh...Hey I'm sorry officer, I - uhh..  
> **Darci:** Get the hell outta here, sicko.  
> **Flasher:** Yes'm, I'm gone!  

- **Darci:** Hmmm, this is some pissy hideout! Looks like nobody's home though.... *(on entering an area)*

*Conversation (on talking to Cabbie):*

> **Cabbie:** You wanna cab?  
> **Darci:** Stow it, pal.  

- **Bonus:** Good work Stern - I guess we'll have to stop callin' you Rookie now, huh? Haw haw.. *(when 'Bloody' Mary Schwartz is arrested)*

*Conversation (on entering an area):*

> **Wildcat gossip:** I hear we got some cop bitch on our tail...know anythin' about that?  
> **Wildcat gossip:** Naah not much. I know that reporter, Gordansky, is creepin' TOO CLOSE to our patch!  
> **Wildcat gossip:** Don't worry about that bitch Gordansky, we've got plans for her! Our southside crew...  


*Conversation (on entering an area):*

> **Tip-off citizen:** Officer! I just seen a guy flashin' a gun run down this allyway!  
> **Darci:** Clear the area sir - there's gonna be trouble!  

- **Alien hunter:** I'm looking for alien life forms. WIBBLE! *(on talking to Alien hunter)*
- **Despatch (radio):** Dammit Stern! Your BLOODLUST just cost us a case! *(when 'Bloody' Mary Schwartz is killed (not arrested))*
- **Working girl:** Beat it! I don't do cops! *(on talking to Working girl)*
- **Working girl:** I don't do women, sorry honey! *(on talking to Working girl)*
- **Hot dog vendor:** You wanna hot dog officer? I could BEND it to look like a DOUGHNUT, just so's ya won't feel WIERD...haw haw! *(on talking to Hot dog vendor)*
- **Despatch (radio):** Jeez, Stern - looks like you're still a Rookie... *(when Assault victim is down)*
- **Assault victim:** Thank God! *(on talking to Assault victim)*
- **Cop:** Your doing good rookie, but that don't mean you can get in MY way. *(on talking to Cop)*
- **Cop:** Some uppity damn gang member just tried to hit me with a bat! If I find him, he's gonna BLEED for his COUNTRY! *(on talking to Cop)*
- **Cop:** Hey, this ain't your patch - how come the Chief sent YOU down here? *(on talking to Cop)*

*Conversation (on talking to Roper):*

> **Roper:** Stern! Good to see you're keeping yourself well!  
> **Darci:** I can take care of myself, mystery man. Now what are you doin' following me around?  
> **Roper:** I'm not following you, D'arci Stern - we merely follow the same path.  
> **Darci:** Path? What do you mean path?  
> **Roper:** The WIldcats, Stern - they are more than you think. They are not what they seem. That is why you will need these things I have brought you...  

- **Wildcat (grey) #2:** Ain't no-one told ya copper? This is our turf! An' pretty soon, EVERYWHERE will be OUR turf! Heehee hee! *(when Darci is seen by Wildcat (grey) #2)*
- **Working girl:** Later honey.....I gots my RENT ta earn tonight *(on talking to Working girl)*
- **Working girl:** Hey, I ain't inta ladies, girl! *(on talking to Working girl)*
- **Civilian #381:** This place looks like the pits! *(on talking to Civilian #381)*

*Conversation (after the conversation "HEY! ASSHOLE! YOU WANNA HAND THERE?."):*

> **Working girl:** Sheeit! You cops allways gotta hound us workin' girls, dontcha?  

- **Barmaid:** Sorry honey, just closing up. Try us tomorrow. *(on talking to Barmaid)*
- **Civilian #423:** Heavens above! *(on talking to Civilian #423)*
- **Despatch (radio):** D'arci - We got a definate position for the WildCats' Semtex. You gotta get in there an' blow 'em ta hell, before they come to pick it up! *(at the start)*
- **Wildcat gossip:** Look OUT - A COP! *(after the conversation "I hear we got some cop bitch on our tail...")*

*Conversation (on entering an area):*

> **Spinks:** Forget this, maan. How the hell we supposed to defend ourselves if the cops decide to swoop on this place?  
> **Wildcat guard:** Don' sweat it Spinks - our Man has got this aaall figured out.  
> **Spinks:** Who da hell is "Our Man"?  
> **Wildcat guard:** I don' know an' I don't NEED to know - just like you. So quit bitchin'!  
> **Spinks:** Sheeit...our day BETTER come soon, my man.  

- **Despatch (radio):** Stern, that Ho' at the bar is "Bloody" Mary Schwartz! We want her hauled in in connection with a case we got burnin' - get to it. *(when the player gets near)*
- **Despatch (radio):** Stern - citizen assault - stop it! *(once either: on entering an area; or on entering an area)*
- **Bonus:** Threat neutralized! *(once both: when Wildcat (rasta) #245 is down; and when Wildcat (rasta) #244 is down)*
- **Despatch (radio):** Stern? This is Roper. Meet me at the corner of Geebag Street. I got some things you might need... *(after Roper)*
- **Roper:** D'arci - looks like the Wildcats've brought in a Van to collect - Hutty up an' kill those supplies! *(3s later (once either: "Supplies sabataged!"; or "Good work Stern. But don't get TOO cocky..."))*
- **Wildcat (rasta) #344:** Hey, bitch...we told you cops to STAY DA HELL OUTTA OUR PATCH! Now, you can join our celebrations....! *(after Wildcat (rasta) #344)*
- **Despatch (radio):** Pick up team eliminated. *(once both: once both: when Wildcat (rasta) #396 is down; and when Wildcat (rasta) #395 is down; and once both: when Pick-up team is down; and when Pick-up team is down)*

</details>

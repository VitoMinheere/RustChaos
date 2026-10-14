# Grim Gardens

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\botanicc.ucm`, map `botanicc.iam`, district Botanical Gardens

**Played as:** Darci

**Briefing:**

> Sgt. Bryant was investigating drugs ring downtown for the last 2 months. Three days ago, some scumbag from inside the force tipped off the dealers and quickly stashed the narcotics, worth in excess of $3 million, inside the Botanical Gardens. Word has reached us that gang groups are loitering around the gardens, ready to swoop and take them from under our noses. This is personal between us and them, D'arci - we want to hit them where it hurts - when you find the stashes, destroy the poison!

**Story:** Sergeant Bryant spent two months on a drugs ring until someone inside the force tipped off the dealers, who stashed $3 million of narcotics in the closed Botanical Gardens. Darci gets in (the power, the lifts and the high-pressure jets are on switches), finds explosives, and blows both narcotics stores. Gordansky is there following her own trail: the car bombing wasn't a one-off, the Southside Wildcats are dealing for half a ton of explosives. Other threads: a Wildcat's mother scolding her son ('your daddy was a respectable MUGGER'), the parole-breaker Koots, a dealer selling 'close-to-home-made' product, a gunman who shot a cop outside the hotel. The gardens have plaques: 'Twinned with the town of GUILDFORD'. Arresting or killing Gordansky gets Darci suspended.

**Characters:**

- **Darci Stern**
- **Gordansky**
- **Sergeant Bryant** (mentioned): ran the drugs case
- **A Wildcat and his mother**
- **Koots**: armed felon on parole violation
- **A dealer** and his customer with a balloon
- **The gunman** who shot a cop
- An eight-year veteran cop, a frustrated plumber, gossiping locals

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 45 | Wildcat's mother | large woman | civilian |  |
| 61 | Local | civilian | civilian |  |
| 62 | Local | civilian | civilian |  |
| 66 | Local | civilian | civilian |  |
| 68 | Local | civilian | civilian |  |
| 69 | Local | civilian | civilian |  |
| 75 | Local | civilian | civilian |  |
| 98 | Veteran cop | cop (pistol) | cop |  |
| 106 |  | Wildcat (red) | guard |  |
| 109 | Her son (Wildcat) | Wildcat (grey) | civilian | stands still |
| 151 | Plumber | workman | civilian |  |
| 173 | Local | civilian | civilian |  |
| 219 | Local | civilian | civilian |  |
| 220 | Would-be cop | civilian | civilian |  |
| 221 | Local | civilian | civilian |  |
| 223 | Weedkiller enthusiast | civilian | civilian |  |
| 239 | Koots | Wildcat (red) | guard |  |
| 240 | Gordansky | hostage | civilian | stands still |
| 271 | Woman | woman | civilian |  |
| 273 | Workman | workman | civilian |  |
| 275 | Garden Wildcat | Wildcat (red) | civilian | stands still |
| 276 | Garden Wildcat | Wildcat (grey) | civilian | stands still |
| 298 | Dealer | civilian | civilian | guilty |
| 299 | Customer with a balloon | civilian with a balloon | civilian |  |
| 305 | The gunman | tramp | bully |  |

Others: 9 Wildcat (rasta) (guard); 9 Wildcat (grey) (guard); 7 Wildcat (red) (guard); 5 Wildcat (grey) (assassin); 2 Wildcat (rasta, pistol) (gang member); 1 Wildcat (rasta, pistol) (guard); 1 cop (cop driver); 1 civilian (driver); 1 woman (civilian); 1 cop (a body (dies at once)).

</details>

**Objects:**

- Items: barrels x25, health pack x14, shotgun ammo x6, explosives x5, pistol ammo x3, shotgun
- Vehicles: taxi (locked) x3, police car (unlocked) x2, van (unlocked), ambulance (unlocked)
- Street furniture: bin x30, traffic cone x12
- Set pieces: 14 steam traps, 2 moving platforms

**Badges:** 17. **Side objectives:**

- Side objective (1 badge)
- Arrest the tramp in the south-west (1 badge)
- Arrest the civilian in the west (1 badge)

**Beacons (places and people the player is sent to):** "Power Switch", "Explosives", "Narcotic Stores", "Narcotic stores", "Gang Superior!", "Armed Suspect"

**How it ends:**

- **Won** after "Primary objectives met. Way to go Stern!".
- **Lost** once either: "Stern - what the HELL you doin' arrestin..."; or "Stern you ASSHOLE! YOUVE KILLED A UCNN ...".

<details><summary>Script</summary>

- **Despatch (radio):** Water fountain Active. *(on pressing action here)*
- **Despatch (radio):** I don't want to phone for pizza! *(on pressing action here)*
- **Despatch (radio):** It says "God bless him.. W.J. Clinton." *(on pressing action here)*
- **Despatch (radio):** It says "Dedicated to UNION CITY. Twinned with the town of GUILDFORD. Long may it Rot." *(on pressing action here)*
- **Darci:** Some sort of far eastern tree... *(on pressing action here)*
- **Darci:** It says "This bench is dedicated to the rock group "Last Choking Gasp", Union City's world-conquering sons." Hmm... *(on pressing action here)*

*Conversation (on entering an area):*

> **Her son (Wildcat):** Shut up whore! I ain't listening to you no more!  
> **Wildcat's mother:** Don't you DARE speak to your mother like that!  
> **Her son (Wildcat):** I - I'm sorry momma, but - but I gotta go meet the guys!  
> **Wildcat's mother:** Those bums? You keep hangin' with them, an' you'll end up DEAD! You're daddy was a respectable MUGGER - now why you want to go messin' with those Wildcats?  


*Conversation (on entering an area):*

> **Garden Wildcat:** Those dumbass cops - the boss was right to think they'd never find us in the gardens!  
> **Garden Wildcat:** Yeah, but for how long? Sooner or later they're gonna come creepin', man...  


*Conversation (on entering an area):*

> **Gordansky:** Officer Stern?  
> **Darci:** Gordansky! What are YOU doing here?  
> **Gordansky:** I got my trail to follow, just like you, Stern. And my trail led me here, just like yours.  
> **Darci:** So, you find out anythin' more on the Wildcats? The chief still thinks they're a bunch of dumb street scum, but I don't know...  
> **Gordansky:** Well, remember that car bombing incident a few days back?  
> **Darci:** Hell yeah I remember! I was in the middle of ALL the shit!  
> **Gordansky:** Well, that wasn't just a ONE-OFF! Word I hear is, the 'cats are plannin' a deal for a HALF-TON of explosives!  
> **Darci:** What do you know of this? Where is it?  
> **Gordansky:** All I know is, it's the SOUTHSIDE Wildcats who're taking this deal through.  
> **Darci:** The SOUTHSIDE, hunh? They're the ones who grabbed you, ain't they?  
> **Gordansky:** the same scumrags, yeah.  
> **Darci:** Thanks, Gordansky. One day Ill give you an exclusive...!  

- **Despatch (radio):** Loading lift on. *(once both: on pressing action here; and on pressing action here)*
- **Wildcat's mother:** Get away from me, COP! *(on talking to Wildcat's mother)*
- **Local:** Pity the gardens have been closed for weeks. Don't NOTHIN' work in this stinking city? *(on talking to Local)*
- **Local:** Officer, Somethin' strange is goin' on behind those walls... *(on talking to Local)*
- **Local:** What's that smell..? Mmmmm.... *(on talking to Local)*
- **Local:** The roads round here need work. *(on talking to Local)*
- **Local:** It smells like....like....MOIDAH! *(on talking to Local)*
- **Local:** Snf? *(on talking to Local)*
- **Veteran cop:** I been on the beat 8 years - so don't talk down to me! *(on talking to Veteran cop)*

*Conversation (after the conversation "Shut up whore! I ain't listening to you ..."):*

> **Darci:** You better listen to your mother good, boy!  
> **Her son (Wildcat):** What the - the COPS!  

- **Plumber:** This goddamn pipe is busted! I'll NEVER get it fixed! *(on talking to Plumber)*
- **Despatch (radio):** We got two stores of narcotics, in and around the Botanical gardens. *(at the start)*
- **Despatch (radio):** ...find some plastic and blow 'em sky high! *(at the start)*
- **Local:** I've seen people with guns in these parts. *(on talking to Local)*
- **Wildcat's mother:** DON'T YOU TOUCH MY BOY! *(when Her son (Wildcat) is down)*
- **Woman:** What you want from me? *(on talking to Woman)*
- **Despatch (radio):** Primary objectives met. Way to go Stern! *(once both: at the start; and at the start)*
- **Would-be cop:** HEY, A COP! I'm thinkin' about joinin' up...Whaddaya think? *(on talking to Would-be cop)*
- **Local:** Asshole Wildcats.. they're tearing this area apart! *(on talking to Local)*
- **Weedkiller enthusiast:** Hey! I read on the internet that Weed Killer makes a good explosive! That true Officer? Is it? Huh? *(on talking to Weedkiller enthusiast)*
- **Local:** Man, somethin' ain't done soon, we'll ALL be joinin' the gangs. *(on talking to Local)*
- **Workman:** Hey lady, this AIN'T no alley for wannabe cops. You best scoot. *(on talking to Workman)*
- **Despatch (radio):** High pressure jets off. *(on pressing action here)*
- **Bonus:** Drug dealer arrested. *(when Dealer is arrested)*

*Conversation (when Dealer is seen by Darci):*

> **Dealer:** Man, I TOLD ya I be supplyin' the SWEETEST product  
> **Customer with a balloon:** Cool. How much?  
> **Dealer:** Twenty bucks each.  
> **Customer with a balloon:** Arright. Hey, where d'you get this shit from?  
> **Dealer:** Lets just say it's close-to-home-made.  

- **Gordansky:** If you still need to get in, try around the back. *(on talking to Gordansky)*
- **Despatch (radio):** Stern - what the HELL you doin' arrestin' a REPORTER? This'll KILL the force at City Hall! Dammit, your ass is suspended! *(when Gordansky is arrested)*
- **Despatch (radio):** Stern you ASSHOLE! YOUVE KILLED A UCNN REPORTER?!? Your ass is SUSPENDED - get back here an' hand in your badge! *(when Gordansky is killed (not arrested))*
- **Darci:** Shit! Power's off... *(on pressing action here)*

*Conversation (after the conversation "You better listen to your mother good, b..."):*

> **Wildcat's mother:** You LEAVE my BOY ALONE, lady! I'll BUST YOUR ASS if you TOUCH him!  
> **Darci:** If he's in the Wildcats, I need to talk to him....  

- **Darci:** Shit! I ain't lettin' that dealer scum go... *(after the conversation "Man, I TOLD ya I be supplyin' the SWEETE...")*

*Conversation (after the conversation "You LEAVE my BOY ALONE, lady! I'll BUST ..."):*

> **Her son (Wildcat):** You're DEAD, hag!  

- **Despatch (radio):** All units - Patrol car sighted a known Felon, known as "Koots", on a parole violation. Known to be armed and dangerous. We want him brought down! *(after Koots)*
- **Despatch (radio):** You got him Stern! Keep 'em comin'! *(when Koots is arrested)*
- **Despatch (radio):** Stern! We jus' had a cop downed outside the hotel - the scumbag's still at large! Beat his ass an' haul him in for the guys at the station..!! *(20s later (after "Officer, Somethin' strange is goin' on b..."))*
- **Despatch (radio):** Gunman arrested. *(when The gunman is arrested)*
- **Despatch (radio):** Damn Stern, did you have to kill him? Sheeit.. *(when The gunman is killed (not arrested))*

</details>

# Semtex *(draft; `memtex`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\memtex.ucm`, map `snocrime.iam`

**Played as:** Darci

**Story:** Semtex almost as shipped.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 6 | Deeks | tramp | civilian | stands still |
| 26 | Wildcat hunting Deeks | Wildcat (rasta, pistol) | civilian | stands still |
| 28 | Wildcat | Wildcat (red) | civilian | stands still |
| 80 | Officer Molko | cop (pistol) | cop | invulnerable |
| 87 | Roper | Roper | civilian | stands still |
| 142 |  | Wildcat (grey) | bully |  |
| 143 |  | Wildcat (rasta) | guard |  |
| 161 | Southside Wildcat | Wildcat (red) | guard |  |
| 162 | Southside Wildcat | Wildcat (red) | guard |  |
| 166 | Wildcat | Wildcat (grey) | bully | stands still |
| 171 | Wildcat messenger | Wildcat (rasta) | guard |  |
| 189 | Southside woman | woman | guard | guilty, stands still |
| 194 | Wildcat | Wildcat (red) | guard |  |
| 196 |  | Wildcat (red) | guard |  |
| 206 | Semtex guard | Wildcat (grey, AK-47) | guard | stands still |
| 207 | Wildcat | Wildcat (grey) | Man In Black |  |
| 208 | Wildcat | Wildcat (grey) | guard |  |
| 223 | Sergeant Smyth | cop (shotgun) | guard | guilty, stands still |
| 225 |  | Wildcat (grey) | assassin |  |
| 243 | Roper | Roper | civilian | stands still |
| 274 | Wildcat on the roof | Wildcat (red) | guard | stands still |
| 288 | Workman lookout | workman | civilian |  |
| 313 |  | Wildcat (rasta) | guard |  |
| 338 | Cop | cop (pistol) | cop | stands still |
| 340 | Wildcat | Wildcat (red) | guard |  |
| 348 | Man on the roof | civilian | stands about | stands still |

Others: 16 Wildcat (red) (guard); 12 Wildcat (grey) (guard); 4 Wildcat (rasta) (guard); 3 Wildcat (rasta, pistol) (guard); 3 Wildcat (red, shotgun) (guard); 3 workman (guard); 3 cop (pistol) (cop); 2 Wildcat (grey) (assassin); 2 Wildcat (rasta, pistol) (driver); 2 civilian (driver); 2 workman (assassin); 2 Wildcat (red) (assassin); 1 Wildcat (red) (kills a colour); 1 Wildcat (rasta, pistol) (bully); 1 Wildcat (rasta) (assassin); 1 Wildcat (red, shotgun) (assassin); 1 Wildcat (rasta, pistol) (assassin); 1 Wildcat (grey) (bully); 1 Wildcat (rasta) (bully); 1 Wildcat (rasta) (stands about); 1 Wildcat (grey) (civilian).

</details>

**Objects:**

- Items: health pack x17, barrels x7, shotgun ammo x3, pistol x2, pistol ammo x2, AK-47 ammo, shotgun
- Vehicles: van (unlocked) x2, sedan (unlocked), car (unlocked)
- Street furniture: barrel x8, oil drum x3
- Set pieces: 2 mist banks

**Badges:** 24. **Side objectives:**

- Side objective (1 badge)
- Talk to the civilian on a rooftop in the north-west (2 badges)

**Beacons (places and people the player is sent to):** "Deeks.", "Officer Molko", "Roper", "Roper (equip.)", "Semtex!", "Hostage"

**How it ends:**

- **Lost** 2s later (after "Shit! Looks like what Deeks said about h...").
- **Lost** when Roper is killed (not arrested).
- **Won** 6s later (once both: when the barrel #199 blows up; and when the barrel #183 blows up).

<details><summary>Script</summary>

- **Despatch (radio):** Stern? Snitch says the Wildcats are plannin' to TAKE OVER this patch of UC! Reports are comin' in from ALL OVER of armed thugs creepin' about the streets. We want their asses HAULED, Stern! *(at the start)*
- **Wildcat:** Come any closer an' I'll blast ya! I swear ta God I will! *(on entering an area)*
- **Wildcat:** You best get out of here or we're gonna blow ya all to HELL! *(on entering an area)*
- **Wildcat:** Give it up, COP SCUM! Don't you know we cats is wild?? Didnt your mamma done TOLD YA? YEEHAAH! *(on entering an area)*

*Conversation (on entering an area):*

> **Man on the roof:** HEEELP MEEEeeee!  
> **Wildcat on the roof:** Aw SHUT IT fly-boy! There ain't nobody listening down there - WILDCATS rule da streets now.  
> **Man on the roof:** Heeeelp  
> **Wildcat on the roof:** Ain't you got it yet kid? You ain't payin' your DUES, the Boss says you gotta FLY!  
> **Man on the roof:** ...h-help? Ulp!  

- **Darci:** Roger Sarge. I gotta meet my informant Deeks up at the Gas Station first - he has some juice for me. If I see any Wildcats on the way - consider their asses hauled. D'arci out. *(8s later (at the start))*
- **Darci:** Shit! Looks like what Deeks said about his life being in danger was true...and damn if I didn't need that info! *(when Deeks is down)*

*Conversation (on talking to Deeks):*

> **Darci:** Arright Deeks - why'd those guys attack us? Was it you they were after?  
> **Deeks:** D'arci, the Wildcats are being controlled from up on high. Guy in City Hall pulls a string, an' another body drops dead in Union City!  
> **Darci:** Nice story, but evidence Deeks, I need evidence!  
> **Deeks:** Then get your ass over to 11th Street an' talk to Officer Molko - me, I'm splittin' before I get wacked!  
> **Darci:** Molko huh? Thanks Deeks, you best lay low for a while.  


*Conversation (on entering an area):*

> **Southside Wildcat:** Weeell - a little lost cop! Didn't nobody warn ya honey? There's a new order in town - we're TAKIN' OVER from the UCPD!  
> **Darci:** Yeah, reporter friend of mine warned me that you Southside Wildcats were plannin' somethin' stupid...now what asshole plan was it she said again?  
> **Southside Wildcat:** Wildcats know who you are Stern. We KNOW you followin' us, an' we're here to CUT you down!  
> **Darci:** We..?  
> **Southside Wildcat:** Say hello to the lady, Spider...  

- **Wildcat:** We're puttin' you outta BUSINESS, cop! *(when Darci is seen by Wildcat)*
- **Southside woman:** Hey you PIG! This ain't your neighborhood! *(on talking to Southside woman)*
- **Despatch (radio):** Got the bastards!! *(when the whole group is down)*
- **Wildcat:** Last chance to get outta here with your ARMS still attached cop!! *(when Darci is seen by Wildcat)*
- **Despatch (radio):** Assholes down! Now I gotta talk to Deeks - find out what the hell's goin' on! *(when the whole group is down)*
- **Wildcat:** The order's gone out, cop - we Wildcats Got to kill one COP a day! HELLO QUOTA! *(when Darci is seen by Wildcat)*

*Conversation (after the conversation "Weeell - a little lost cop! Didn't nobod..."):*

> **Southside Wildcat:** Hiiiii, copper. How's it feel, now you're about to die - in a doomed city?  
> **Darci:** Try it, asshole - I already messed up LEGIONS of Wildcats these last few weeks, two more won't kill me...  
> **Southside Wildcat:** They just might, She-Pig!  


*Conversation (when Darci is seen by Wildcat messenger):*

> **Wildcat messenger:** HEY STERN!  
> **Darci:** Huhn? Do I know you, scumbag?  
> **Wildcat messenger:** We gotta message for ya, Darci Stern.. why don't you come over here an' get it?  
> **Darci:** Oh yeah? What message is that?  
> **Wildcat messenger:** One written in COLD STEEL, whore!  

- **Roper:** D'arci - this is Roper.. I'm waiting across town for you if you need any extra equipment. Good luck at the Docks - over! *(once both: on entering an area; and when the whole group is down)*

*Conversation (when Darci is seen by Semtex guard):*

> **Semtex guard:** I got me a GRENADE, hag! I'll blow us all to HELL before I turn this Semtex over to you!  

- **Despatch (radio):** AAAGH! HEEELP! He's gonna KILL ME! *(on entering an area)*

*Conversation (on talking to Man on the roof):*

> **Darci:** You okay sir?  
> **Man on the roof:** Jeez - that nut was gonna kill me! I  really think he was gonna PUSH ME OFF!  
> **Darci:** Okay - you best get going - I'm afraid I got some real pressing business to get on with, so...  
> **Man on the roof:** Oh sure Officer, I'll get out of your way. Thanks again, Officer ehh...  
> **Darci:** ..Stern.  
> **Man on the roof:** Stern - thank you!  


*Conversation (once both: on entering an area; and when Darci is seen by Deeks):*

> **Darci:** That you Deeks?  
> **Deeks:** D'arci! Shit, where the hell you been? I could get POPPED just bein' SEEN on the streets!  
> **Darci:** Calm down, Deeks! What's goin' on?  
> **Deeks:** Aw man, what AIN'T goin on? This is BIG Stern - bigger than...  


*Conversation (on talking to Officer Molko):*

> **Officer Molko:** Officer Stern ain't it?  
> **Darci:** Yeah Molko. Lissen, I just got done talkin' to Deeks - says you have some info for me?  
> **Officer Molko:** Not me, D'arci, someone else does, a friend of both of ours...  
> **Darci:** Quit foolin', Molko - what "friend"?  

- **Cop:** Yeah Stern, Officer Molko's on his beat - go down the street behind me and you should find him. *(on talking to Cop)*
- **Despatch (radio):** Dammit Stern - couldn't you even save some poor bum from gettin' thrown off a ROOF? *(when Man on the roof is killed (not arrested))*
- **Wildcat on the roof:** Get flappin' buddy... its a long way DOWN!! *(120s later (earlier events))*
- **Despatch (radio):** Way to go Stern - looks like the Wildcats are gonna have to revise their plans. You've saved countless lives by blowing that Semtex to hell! *(once both: when the barrel #199 blows up; and when the barrel #183 blows up)*

*Conversation (once both: on entering an area; and when Darci is seen by Sergeant Smyth):*

> **Sergeant Smyth:** STERN I'M WARNIN' YA! STAY BACK OR I'LL SHOOT!  
> **Darci:** Sergeant Smyth? Is that you??  
> **Sergeant Smyth:** Turn around Stern - just walk away from this!  
> **Darci:** I ain't walkin' NOWHERE! So you're the one who's been tippin' off the Wildcats about our busts..!  
> **Sergeant Smyth:** Listen, I ain't gonna warn ya again - GET THE HELL OUTTA HERE!  
> **Darci:** You've gone rogue, Smyth.... and I'm taking you down!  


*Conversation (once both: at the start; and on talking to Officer Molko):*

> **Darci:** Molko..  
> **Officer Molko:** You still here? Dammit D'arci, go talk to Roper - he'll tell you anything you need to know.  


*Conversation (on talking to Roper):*

> **Darci:** I'm just about beat. Can you help some?  
> **Roper:** Sure. Oh, and one more tip - I hear that the Wildcats are plannin' to go to all out war with the UCPD.  
> **Darci:** The whole force? Are you nuts?  
> **Roper:** Way I hear it, they're importin' more explosives to bomb cop stations, and are even gonna KIDNAP selected cops!  
> **Darci:** I think you got some informants with mental problems Roper. Not even the Wildcats are that dumb!  
> **Roper:** We'll see, girl.  

- **Wildcat hunting Deeks:** .. bigger than the turnout at your FUNERAL, Deeks you RAT! Thought us Wildcats wouldn't catch up with you, huh? *(0s later (after the conversation "That you Deeks?"))*
- **Bonus:** Got him! *(when Wildcat is arrested)*
- **Darci:** Gotcha, chump! *(when Wildcat hunting Deeks is arrested)*

*Conversation (on talking to Roper):*

> **Darci:** Roper - what the hell is going on?  
> **Roper:** I notice you've been catching up on the Wildcats - and you'll have noticed the City is being slowly overrun!  
> **Darci:** Hell Roper, Union City is falling into ANARCHY! The Wildcats are runnin' around like there's no such thing as COPS!!  
> **Roper:** And the COPS are exactly who they're going after. They plan to rid this city of the UCPD for GOOD.  
> **Darci:** What's behind this Roper...what's all this to you?  
> **Roper:** The Wildcats are just the tip of all this, D'arci - this is bigger and more INSANE than you might believe...  
> **Darci:** Why not take it to the UCPD. You got any proof?  
> **Roper:** That's what I need you for, D'arci. Only you and your badge can discover what I need! The UCPD are beyond help.. It's down to you and me!  
> **Darci:** Roper, I ain't sure about this...  
> **Roper:** Your father would tell you - this task is vastly bigger than my death or yours, D'arci. So the question is - are you WITH ME? If you are, take this SHOTGUN.......  


*Conversation (when Roper gets near):*

> **Roper:** Stern! We need to talk....  
> **Darci:** YOU??  

- **Roper:** Move it, D'arci! *(when Roper gets near)*

*Conversation (once both: 4s later (when the whole group is down); and when Darci is seen by Roper):*

> **Roper:** You DID it! I knew I was right about You! Now we gotta split up, you got work needs doin down the docks!  
> **Darci:** WAITAMINUTE! Ain't you gonna explain yourself!  
> **Roper:** Later - for now, the Wildcats have 2 barrels of SEMTEX-fuelled chaos that Bane is figuring will save his election campaign!  
> **Darci:** Semtex? Where?  
> **Roper:** They got it hidden in two barrels in the dock land section of town! You've gotta get to the docks and destroy that explosive!  
> **Darci:** How the hell do I do that?  
> **Roper:** One shot oughta set the whole thing off - keep the rest o' your bullets for the Wildcats!  
> **Darci:** I got it! Where are you going?  
> **Roper:** We got major trouble with the Wildcats uptown - I'm going to end it. Go careful, D'arci.  

- **Roper:** Hmph! Maybe you inherited your fathers guts after all - now FOLLOW ME! *(on picking up the shotgun #108)*
- **Workman lookout:** Back off, COP!! There ain't NOTHIN' here for you tonight! *(when Darci is seen by Workman lookout)*

</details>

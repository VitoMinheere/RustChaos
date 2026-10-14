# Semtex

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\semtex.ucm`, map `snocrime.iam`, district Southside Beach

**Played as:** Darci

**Briefing:**

> Just go on the beat, but be careful. The underworld seems to be seething with rumours that the Wildcats are planning something BIG for this section of Southside UC. Oh, we also got a call from your snitch, Deeks. Says he wants to meet you to pass on some "personal info". Hell if I know what he means!

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 6 |  | tramp | civilian | stands still |
| 26 |  | Wildcat (rasta, pistol) | civilian | stands still |
| 28 |  | Wildcat (red) | civilian | stands still |
| 80 |  | cop (pistol) | cop | invulnerable |
| 87 |  | Roper | civilian | stands still |
| 142 |  | Wildcat (grey) | bully |  |
| 143 |  | Wildcat (rasta) | guard | stands still |
| 161 |  | Wildcat (red) | guard |  |
| 162 |  | Wildcat (red) | guard |  |
| 166 |  | Wildcat (grey) | bully | stands still |
| 171 |  | Wildcat (rasta) | guard |  |
| 189 |  | woman | guard | guilty, stands still |
| 194 |  | Wildcat (red) | guard |  |
| 196 |  | Wildcat (red) | guard |  |
| 206 |  | Wildcat (grey, AK-47) | guard | stands still |
| 207 |  | Wildcat (grey) | Man In Black |  |
| 208 |  | Wildcat (grey) | guard |  |
| 223 |  | cop (shotgun) | guard | guilty, stands still |
| 225 |  | Wildcat (grey) | assassin |  |
| 243 |  | Roper | civilian | stands still |
| 274 |  | Wildcat (red) | guard | stands still |
| 288 |  | workman | civilian |  |
| 313 |  | Wildcat (rasta) | guard |  |
| 338 |  | cop (pistol) | cop | stands still |
| 340 |  | Wildcat (red) | guard |  |
| 348 |  | civilian | stands about | stands still |

Others: 16 Wildcat (red) (guard); 12 Wildcat (grey) (guard); 5 Wildcat (rasta) (guard); 3 Wildcat (rasta, pistol) (guard); 3 workman (guard); 3 cop (pistol) (cop); 2 Wildcat (grey) (assassin); 2 Wildcat (red, shotgun) (guard); 2 Wildcat (rasta, pistol) (driver); 2 civilian (driver); 2 workman (assassin); 2 Wildcat (red) (assassin); 1 Wildcat (red) (kills a colour); 1 Wildcat (rasta, pistol) (bully); 1 Wildcat (rasta) (assassin); 1 Wildcat (red, shotgun) (assassin); 1 Wildcat (rasta, pistol) (assassin); 1 Wildcat (grey) (bully); 1 Wildcat (rasta) (stands about); 1 Wildcat (grey) (civilian).

</details>

**Objects:**

- Items: health pack x14, barrels x7, pistol ammo x4, shotgun ammo x3, pistol x2, AK-47 ammo, shotgun
- Vehicles: van (unlocked) x2, sedan (unlocked), car (unlocked)
- Street furniture: barrel x6, oil drum x2
- Set pieces: 2 mist banks

**Badges:** 27. **Side objectives:**

- Side objective (1 badge)
- Talk to the civilian on a rooftop in the north-west (2 badges)

**Beacons (places and people the player is sent to):** "Deeks.", "Officer Molko", "Roper", "Roper (equip.)", "Semtex!", "Hostage"

**How it ends:**

- **Lost** 2s later (after "Shit! Looks like what Deeks said about h...").
- **Lost** when Roper #87 is killed (not arrested).
- **Won** 6s later (once both: when the oil drum #199 blows up; and when the oil drum #183 blows up).

<details><summary>Script</summary>

- **Despatch (radio):** Stern? Snitch says the Wildcats are plannin' to TAKE OVER this patch of UC! Reports are comin' in from ALL OVER of armed thugs creepin' about the streets. We want their asses HAULED, Stern! *(at the start)*
- **Wildcat (grey) #208:** Come any closer an' I'll blast ya! I swear ta God I will! *(on entering an area)*
- **Wildcat (red) #194:** You best get out of here or we're gonna blow ya all to HELL! *(on entering an area)*
- **Wildcat (red) #194:** Give it up, COP SCUM! Don't you know we cats is wild?? Didnt your mamma done TOLD YA? YEEHAAH! *(on entering an area)*

*Conversation (on entering an area):*

> **Civilian #348:** HEEELP MEEEeeee!  
> **Wildcat (red) #274:** Aw SHUT IT fly-boy! There ain't nobody listening down there - WILDCATS rule da streets now.  
> **Civilian #348:** Heeeelp  
> **Wildcat (red) #274:** Ain't you got it yet kid? You ain't payin' your DUES, the Boss says you gotta FLY!  
> **Civilian #348:** ...h-help? Ulp!  

- **Darci:** Roger Sarge. I gotta meet my informant Deeks up at the Gas Station first - he has some juice for me. If I see any Wildcats on the way - consider their asses hauled. D'arci out. *(8s later (at the start))*
- **Darci:** Shit! Looks like what Deeks said about his life being in danger was true...and damn if I didn't need that info! *(when Tramp #6 is down)*

*Conversation (on talking to Tramp #6):*

> **Darci:** Arright Deeks - why'd those guys attack us? Was it you they were after?  
> **Tramp #6:** D'arci, the Wildcats are being controlled from up on high. Guy in City Hall pulls a string, an' another body drops dead in Union City!  
> **Darci:** Nice story, but evidence Deeks, I need evidence!  
> **Tramp #6:** Then get your ass over to 11th Street an' talk to Officer Molko - me, I'm splittin' before I get wacked!  
> **Darci:** Molko huh? Thanks Deeks, you best lay low for a while.  


*Conversation (on entering an area):*

> **Wildcat (red) #161:** Weeell - a little lost cop! Didn't nobody warn ya honey? There's a new order in town - we're TAKIN' OVER from the UCPD!  
> **Darci:** Yeah, reporter friend of mine warned me that you Southside Wildcats were plannin' somethin' stupid...now what asshole plan was it she said again?  
> **Wildcat (red) #161:** Wildcats know who you are Stern. We KNOW you followin' us, an' we're here to CUT you down!  
> **Darci:** We..?  
> **Wildcat (red) #161:** Say hello to the lady, Spider...  

- **Wildcat (grey) #166:** We're puttin' you outta BUSINESS, cop! *(when Darci is seen by Wildcat (grey) #166)*
- **Woman #189:** Hey you PIG! This ain't your neighborhood! *(on talking to Woman #189)*
- **Despatch (radio):** Got the bastards!! *(when the whole group is down)*
- **Wildcat (grey) #207:** Last chance to get outta here with your ARMS still attached cop!! *(when Darci is seen by Wildcat (grey) #207)*
- **Despatch (radio):** Assholes down! Now I gotta talk to Deeks - find out what the hell's goin' on! *(when the whole group is down)*
- **Wildcat (red) #340:** The order's gone out, cop - we Wildcats Got to kill one COP a day! HELLO QUOTA! *(when Darci is seen by Wildcat (red) #340)*

*Conversation (after the conversation "Weeell - a little lost cop! Didn't nobod..."):*

> **Wildcat (red) #162:** Hiiiii, copper. How's it feel, now you're about to die - in a doomed city?  
> **Darci:** Try it, asshole - I already messed up LEGIONS of Wildcats these last few weeks, two more won't kill me...  
> **Wildcat (red) #162:** They just might, She-Pig!  


*Conversation (when Darci is seen by Wildcat (rasta) #171):*

> **Wildcat (rasta) #171:** HEY STERN!  
> **Darci:** Huhn? Do I know you, scumbag?  
> **Wildcat (rasta) #171:** We gotta message for ya, Darci Stern.. why don't you come over here an' get it?  
> **Darci:** Oh yeah? What message is that?  
> **Wildcat (rasta) #171:** One written in COLD STEEL, whore!  

- **Roper #87:** D'arci - this is Roper.. I'm waiting across town for you if you need any extra equipment. Good luck at the Docks - over! *(once both: on entering an area; and when the whole group is down)*

*Conversation (when Darci is seen by Wildcat (grey, AK-47) #206):*

> **Wildcat (grey, AK-47) #206:** I got me a GRENADE, hag! I'll blow us all to HELL before I turn this Semtex over to you!  

- **Despatch (radio):** AAAGH! HEEELP! He's gonna KILL ME! *(on entering an area)*

*Conversation (on talking to Civilian #348):*

> **Darci:** You okay sir?  
> **Civilian #348:** Jeez - that nut was gonna kill me! I  really think he was gonna PUSH ME OFF!  
> **Darci:** Okay - you best get going - I'm afraid I got some real pressing business to get on with, so...  
> **Civilian #348:** Oh sure Officer, I'll get out of your way. Thanks again, Officer ehh...  
> **Darci:** ..Stern.  
> **Civilian #348:** Stern - thank you!  


*Conversation (once both: on entering an area; and when Darci is seen by Tramp #6):*

> **Darci:** That you Deeks?  
> **Tramp #6:** D'arci! Shit, where the hell you been? I could get POPPED just bein' SEEN on the streets!  
> **Darci:** Calm down, Deeks! What's goin' on?  
> **Tramp #6:** Aw man, what AIN'T goin on? This is BIG Stern - bigger than...  


*Conversation (on talking to Cop (pistol) #80):*

> **Cop (pistol) #80:** Officer Stern ain't it?  
> **Darci:** Yeah Molko. Lissen, I just got done talkin' to Deeks - says you have some info for me?  
> **Cop (pistol) #80:** Not me, D'arci, someone else does, a friend of both of ours...  
> **Darci:** Quit foolin', Molko - what "friend"?  

- **Workman #288:** Back off, COP!! There ain't NOTHIN' here for you tonight! *(when Darci is seen by Workman #288)*
- **Cop (pistol) #338:** Yeah Stern, Officer Molko's on his beat - go down the street behind me and you should find him. *(on talking to Cop (pistol) #338)*
- **Despatch (radio):** Dammit Stern - couldn't you even save some poor bum from gettin' thrown off a ROOF? *(when Civilian #348 is killed (not arrested))*
- **Wildcat (red) #274:** Get flappin' buddy... its a long way DOWN!! *(120s later (earlier events))*
- **Despatch (radio):** Way to go Stern - looks like the Wildcats are gonna have to revise their plans. You've saved countless lives by blowing that Semtex to hell! *(once both: when the oil drum #199 blows up; and when the oil drum #183 blows up)*

*Conversation (once both: on entering an area; and when Darci is seen by Cop (shotgun) #223):*

> **Cop (shotgun) #223:** STERN I'M WARNIN' YA! STAY BACK OR I'LL SHOOT!  
> **Darci:** Sergeant Smyth? Is that you??  
> **Cop (shotgun) #223:** Turn around Stern - just walk away from this!  
> **Darci:** I ain't walkin' NOWHERE! So you're the one who's been tippin' off the Wildcats about our busts..!  
> **Cop (shotgun) #223:** Listen, I ain't gonna warn ya again - GET THE HELL OUTTA HERE!  
> **Darci:** You've gone rogue, Smyth.... and I'm taking you down!  


*Conversation (once both: at the start; and on talking to Cop (pistol) #80):*

> **Darci:** Molko..  
> **Cop (pistol) #80:** You still here? Dammit D'arci, go talk to Roper - he'll tell you anything you need to know.  


*Conversation (on talking to Roper #243):*

> **Darci:** I'm just about beat. Can you help some?  
> **Roper #243:** Sure. Oh, and one more tip - I hear that the Wildcats are plannin' to go to all out war with the UCPD.  
> **Darci:** The whole force? Are you nuts?  
> **Roper #243:** Way I hear it, they're importin' more explosives to bomb cop stations, and are even gonna KIDNAP selected cops!  
> **Darci:** I think you got some informants with mental problems Roper. Not even the Wildcats are that dumb!  
> **Roper #243:** We'll see, girl.  

- **Wildcat (rasta, pistol) #26:** .. bigger than the turnout at your FUNERAL, Deeks you RAT! Thought us Wildcats wouldn't catch up with you, huh? *(0s later (after the conversation "That you Deeks?"))*
- **Bonus:** Got him! *(when Wildcat (red) #28 is arrested)*
- **Darci:** Gotcha, chump! *(when Wildcat (rasta, pistol) #26 is arrested)*

*Conversation (on talking to Roper #87):*

> **Darci:** Roper - what the hell is going on?  
> **Roper #87:** I notice you've been catching up on the Wildcats - and you'll have noticed the City is being slowly overrun!  
> **Darci:** Hell Roper, Union City is falling into ANARCHY! The Wildcats are runnin' around like there's no such thing as COPS!!  
> **Roper #87:** And the COPS are exactly who they're going after. They plan to rid this city of the UCPD for GOOD.  
> **Darci:** What's behind this Roper...what's all this to you?  
> **Roper #87:** The Wildcats are just the tip of all this, D'arci - this is bigger and more INSANE than you might believe...  
> **Darci:** Why not take it to the UCPD. You got any proof?  
> **Roper #87:** That's what I need you for, D'arci. Only you and your badge can discover what I need! The UCPD are beyond help.. It's down to you and me!  
> **Darci:** Roper, I ain't sure about this...  
> **Roper #87:** Your father would tell you - this task is vastly bigger than my death or yours, D'arci. So the question is - are you WITH ME? If you are, take this SHOTGUN.......  


*Conversation (when Roper #87 gets near):*

> **Roper #87:** Stern! We need to talk....  
> **Darci:** YOU??  

- **Roper #87:** Move it, D'arci! *(when Roper #87 gets near)*

*Conversation (once both: 2s later (when the whole group is down); and when Darci is seen by Roper #87):*

> **Roper #87:** You DID it! I knew I was right about You! Now we gotta split up, you got work needs doin down the docks!  
> **Darci:** WAITAMINUTE! Ain't you gonna explain yourself!  
> **Roper #87:** Later - for now, the Wildcats have 2 barrels of SEMTEX-fuelled chaos that Bane is figuring will save his election campaign!  
> **Darci:** Semtex? Where?  
> **Roper #87:** They got it hidden in two barrels in the dock land section of town! You've gotta get to the docks and destroy that explosive!  
> **Darci:** How the hell do I do that?  
> **Roper #87:** One shot oughta set the whole thing off - keep the rest o' your bullets for the Wildcats!  
> **Darci:** I got it! Where are you going?  
> **Roper #87:** We got major trouble with the Wildcats uptown - I'm going to end it. Go careful, D'arci.  

- **Roper #87:** Hmph! Maybe you inherited your fathers guts after all - now FOLLOW ME! *(on picking up the shotgun #108)*

</details>

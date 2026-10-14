# Semtex *(earliest draft; `test3old`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\test3old.ucm`, map `southside.iam`

**Played as:** Darci

**Story:** The earliest Semtex: Deeks, Molko and Roper.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 6 | Deeks | tramp | civilian |  |
| 26 | Wildcat hunting Deeks | Wildcat (rasta, pistol) | civilian | stands still |
| 27 |  | Wildcat (red) | civilian | stands still |
| 28 | Wildcat | Wildcat (red) | civilian | stands still |
| 80 | Officer Molko | cop (pistol) | cop |  |
| 87 | Roper | Roper | civilian |  |
| 144 |  | Wildcat (grey) | guard |  |
| 161 | Southside Wildcat | Wildcat (red) | guard |  |
| 162 | Southside Wildcat | Wildcat (red) | guard |  |
| 166 | Wildcat | Wildcat (grey) | guard |  |
| 171 |  | Wildcat (grey) | guard |  |
| 189 | Southside woman | woman | civilian | guilty |
| 196 |  | Wildcat (red, shotgun) | guard | stands still |
| 206 | Semtex guard | Wildcat (grey, AK-47) | guard |  |
| 207 | Wildcat | Wildcat (grey) | guard |  |
| 208 | Wildcat | Wildcat (grey) | guard |  |
| 223 | Sergeant Smyth | cop (shotgun) | guard | stands still |

Others: 9 Wildcat (grey) (guard); 7 Wildcat (red, shotgun) (guard); 2 Wildcat (rasta) (guard); 2 Wildcat (rasta, pistol) (guard); 1 Wildcat (red) (guard); 1 workman (guard); 1 Wildcat (grey) (bully).

</details>

**Objects:**

- Items: health pack x11, barrels x8, pistol ammo x3, shotgun, mine, wire cutters

**Badges:** 34

**Beacons (places and people the player is sent to):** "Deeks.", "11th Avenue (Molko)", "Roper"

**How it ends:**

- **Lost** 2s later (after "Shit! Looks like wgat Deeks said about h...").
- **Lost** when Roper is killed (not arrested).
- **Won** when a prop is damaged.

<details><summary>Script</summary>

- **Despatch (radio):** Stern? Snitch says the Wildcats are plannin' to take over this southside of UC. Reports are comin' in from ALL OVER of armed thugs creepin' about the streets. We want their asses HAULED, Stern! *(at the start)*
- **Despatch (radio):** 11th Avenue and Santino *(when the player gets near)*
- **Despatch (radio):** 5th Street *(when the player gets near)*
- **Despatch (radio):** 5th Street and Tenson *(when the player gets near)*
- **Despatch (radio):** 11th Avenue *(when the player gets near)*
- **Despatch (radio):** Skarpers Beach *(when the player gets near)*
- **Despatch (radio):** Dockside Bridge *(when the player gets near)*
- **Despatch (radio):** West Freeway and 8th Street *(when the player gets near)*
- **Despatch (radio):** 8th Street and Santino *(when the player gets near)*
- **Despatch (radio):** 8th Street *(when the player gets near)*
- **Despatch (radio):** tenson road *(when the player gets near)*
- **Despatch (radio):** West Freeway *(when the player gets near)*
- **Despatch (radio):** 8th Street and Tenson *(when the player gets near)*
- **Despatch (radio):** Tenson Road *(when the player gets near)*
- **Despatch (radio):** 5th Street *(when the player gets near)*
- **Despatch (radio):** tenson Road *(when the player gets near)*
- **Despatch (radio):** 11th Avenue *(when the player gets near)*
- **Despatch (radio):** 11th Avenue *(when the player gets near)*
- **Despatch (radio):** Santino Drive *(when the player gets near)*
- **Despatch (radio):** Santino Drive *(when the player gets near)*
- **Despatch (radio):** 11th Avenue *(when the player gets near)*
- **Despatch (radio):** 11th Avenue and 5th Street *(when the player gets near)*
- **Despatch (radio):** 11th Avenue *(when the player gets near)*
- **Southside woman:** HEY YOU PIG! THIS AIN'T YOUR NEIGHBOURHOOD! GET OUTTA HERE, COP BITCH! *(on entering an area)*
- **Darci:** Roger Sarge. I gotta meet my informant Deeks up at the West Freeway Gas Station first - has some juice for me. If I see any Wildcats on the way - consider their asses hauled. D'arci out. *(8s later (after "Stern? Snitch says the Wildcats are plan..."))*

*Conversation (on entering an area):*

> **Darci:** That you Deeks?  
> **Deeks:** D'arci! Jeezus Christ, where the hell you been? I could get POPPED just bein' SEEN on the streets!  
> **Darci:** What you mean? What's goin' on?  
> **Deeks:** Aw man, what AIN'T goin on? This is BIG Stern - bigger than...  

- **Darci:** Shit! Looks like wgat Deeks said about his life being in danger was tru...and dman if I didn't need that info! So where to now....? *(when Deeks is killed (not arrested))*

*Conversation (on talking to Deeks):*

> **Darci:** Arright Deeks - why'd those guys attack us? Was it you they were after?  
> **Deeks:** D'arci, the Wildcats are being controlled from up on high. Guy in City Hall pulls a string, an' another body drops dead in Union City!  
> **Darci:** Nice story, but evidence Deeks, I need evidence!  
> **Deeks:** Then get your ass over to 11th Street an' talk to Officer Molko - Me, I'm splittin' before I get wacked!  
> **Darci:** Molko huh? Thanks.  


*Conversation (when "Stern! We need to talk...." gets near):*

> **Roper:** Stern! We need to talk....  
> **Darci:** YOU??  

- **Wildcat (grey) #144:** You best stay back, Bitch - and take that old fart with you. Wildcats got business to take care of! *(on entering an area)*

*Conversation (when the whole group is down):*

> **Roper:** You DID it! I knew I was right about you...now come on, there's work needs doin' down the docks!  
> **Darci:** WAITAMINUTE! You think I'll follow you like a toy dog? Ain't you gonna explain yourself!  
> **Roper:** Later - for now, we got a barrel of SEMTEX that Bane is figurin's gonna save his ELECTION CAMPAIGN!  
> **Darci:** Semtex? SHIT! Where?  
> **Roper:** I gotta clear up a BIG mess uptown - You, get to the docks and destroy that barrel!  
> **Darci:** How the hell do I do that?  
> **Roper:** One shot oughta set the whole thing off - keep the rest o' your bullets for the WIldcats!  
> **Darci:** I got it! Where you going?  
> **Roper:** We got major trouble with the Wildcats uptown - I'm going to end it. Go careful, D'arci.  


*Conversation (on entering an area):*

> **Southside Wildcat:** Weell - a little lost cop! Din't nobody warn ya honey? There's a new order in town - an' it AIN'T the UCPD!  
> **Darci:** Izzat so? Why don't you come down off that roof an' I'll show you ALL about ORDER..  
> **Southside Wildcat:** Well, maybe I jes' might DO that bitch - ain't that so, billy-bob?  

- **Wildcat:** Hey Bitch! You an' your damn COPS're gonna GET YOURS! Get BACK here...! *(when Darci is seen by Wildcat)*

*Conversation (on entering an area):*

> **Wildcat (grey) #171:** HEY BLUE BLOOD!  
> **Darci:** Whu..?  
> **Wildcat (grey) #171:** Hurr hurr...We gotta message for ya...why don't ya come over here an' get it?  
> **Darci:** Oh yeah? What kinda message is that, scumbag?  
> **Wildcat (grey) #171:** It's written in cold STEEL, bitch!  

- **Despatch (radio):** Got the bastards!! *(when the whole group is down)*
- **Bonus:** You did good, Girl! *(when the whole group is down)*

*Conversation (on entering an area):*

> **Sergeant Smyth:** STERN I'M WARNIN' YA! STAY BACK OR I'LL SHOOT!  
> **Darci:** Sergeant Smith? Is that you??  
> **Sergeant Smyth:** Turn around Stern - just walk away from this!  
> **Darci:** I ain't walkin' NOWHERE! So you're the one who's been tippin' off the Wildcats about our busts..!  
> **Sergeant Smyth:** Lissen, I ain't gonna warn ya again - GET THE HELL OUTTA HERE!  
> **Darci:** Smith......I'm taking you DOWN!  


*Conversation (after the conversation "Weell - a little lost cop! Din't nobody ..."):*

> **Southside Wildcat:** Suuure is, Falco - I think this here COP don' know the trouble she's in! No biggie, I'll just fill 'er in...  
> **Darci:** Try it, asshole - I already messed up LEGIONS o' Wildcats these last few weeks, two more won't kill me..  
> **Southside Wildcat:** They might you know....  


*Conversation (on talking to Officer Molko):*

> **Darci:** Officer Stern ain't it?  
> **Officer Molko:** Yeah Molko. Lissen, I just got done talkin' to Deeks - says you have some info for me?  
> **Darci:** Not me, D'arci, soemeone else - a friend of both of ours...  
> **Officer Molko:** Quit foolin', Molko - what "friend"?  

- **Wildcat hunting Deeks:** Weell, lookie here - if it ain't DEEKS an' 'is BITCH COP, just like the boss said... *(0s later (after the conversation "That you Deeks?"))*
- **Bonus:** Got him! *(when Wildcat is arrested)*
- **Bonus:** Secondary objective complete. 200 points. *(when Wildcat (red) #27 is arrested)*
- **Bonus:** Secondary objective complete. 200 points. *(when Wildcat hunting Deeks is arrested)*

*Conversation (on talking to Roper):*

> **Darci:** You again Roper? Lissen, last time we met..  
> **Roper:** D'arci, I ain't got time fer chit-chat! If you're serious about finishin' the Wildcats, take this shotgun. If not...well, I'll know I was wrong about the you..  

- **Roper:** Move it, D'arci! *(when Roper gets near)*
- **Despatch (radio):** Hmph! Seems you got guts - maybe you are your Fathers daughter - NOW LET'S GO! *(1s later (on picking up an item))*

</details>

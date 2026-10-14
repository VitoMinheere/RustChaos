# Gatecrasher

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\bball2.ucm`, map `bball2.iam`, district Baseball Ground

**Played as:** Darci

**Briefing:**

> Stern, we got a tip-off of a meet going down in the grounds of the UC "Black Cats" Stadium. Our snitch says that three gang leaders are meeting to carve up their turf after the recent gang wars in Bushy Park. We need AT LEAST ONE of these gang bosses brought in and worked over, D'arci - the rest, well, we won't complain if you bust a few heads.. heh heh.

**Story:** A tip-off: three gang leaders meet at the UC 'Black Cats' stadium to carve up their turf after the Bushy Park troubles. Georgie, on the gate, says only Wild Bill can let her in. Bill was the baseball team's manager until the Wildcats pushed him out last month, and he's found drinking at Posheeta's Mexican Diner after a detour through a nightclub full of drunks and dancers. Through the back alley Darci gets into the grounds, where Wildcats hold workmen hostage and talk of Banes and AK-47s, and armed men are pinned in a corner by cops. She has to bring a gang boss in alive. Killing the last one ruins the info for Roper.

**Characters:**

- **Darci Stern**
- **Wild Bill**: former 'Black Cats' manager, now a barfly
- **Georgie**: stadium gateman, one of Bill's boys
- **Club doorman**: phones Bill
- **Officer Mills**: stuck on traffic duty
- **The gang bosses** at the meet
- Wildcat guards, hostage workmen, a tramp defending his yard, cops in a standoff, club-goers

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 6 | Diner host | civilian | civilian |  |
| 27 | Gate Wildcat | Wildcat (red) | guard |  |
| 28 | Gate Wildcat | Wildcat (grey) | guard |  |
| 34 | Club-goer | civilian | civilian |  |
| 73 |  | Wildcat (rasta) | guard |  |
| 74 | Wildcat (Banes' runner) | Wildcat (red) | guard |  |
| 75 | Wildcat supplier | Wildcat (red) | guard |  |
| 157 | Georgie | workman | cop |  |
| 158 |  | workman | cop |  |
| 161 |  | civilian | civilian |  |
| 162 |  | civilian | civilian |  |
| 165 |  | civilian | civilian |  |
| 166 |  | civilian | civilian |  |
| 167 |  | civilian | civilian |  |
| 175 |  | tramp | civilian | stands still |
| 181 | Wildcat's girlfriend | woman | civilian | stands still |
| 184 | Wildcat with his girl | Wildcat (grey) | civilian | stands still |
| 188 | Wild Bill | tramp | civilian | stands still |
| 210 |  | civilian | civilian |  |
| 211 |  | civilian | civilian |  |
| 212 |  | civilian | civilian |  |
| 213 | Club doorman | workman | civilian | stands still |
| 214 |  | civilian | civilian |  |
| 215 |  | civilian | civilian |  |
| 216 |  | civilian | civilian |  |
| 217 |  | civilian | civilian |  |
| 218 |  | civilian | civilian |  |
| 219 |  | civilian | civilian |  |
| 220 |  | civilian | civilian |  |
| 221 |  | civilian | civilian |  |
| 222 |  | civilian | civilian |  |
| 223 |  | civilian | civilian |  |
| 224 |  | civilian | civilian |  |
| 225 |  | civilian | civilian |  |
| 226 |  | civilian | civilian |  |
| 227 |  | civilian | civilian |  |
| 228 |  | civilian | civilian |  |
| 230 |  | civilian | civilian |  |
| 258 |  | Wildcat (red) | bully |  |
| 278 | Officer Mills | cop (pistol) | cop |  |
| 294 |  | Wildcat (grey) | guard |  |
| 295 |  | Wildcat (grey) | guard |  |
| 297 | Hostage workman | workman | civilian | stands still |
| 298 | Wildcat guard | Wildcat (red) | guard |  |
| 325 | Cop at the standoff | cop | cop |  |
| 326 |  | Wildcat (red) | guard |  |
| 327 |  | Wildcat (red) | guard |  |
| 328 | Cop at the standoff | cop | cop |  |
| 329 | Cop at the standoff | cop | cop |  |
| 356 | Tramp in his yard | tramp | civilian |  |
| 368 | Senior cop | cop (pistol) | cop |  |

Others: 20 Wildcat (grey) (guard); 11 Wildcat (rasta) (guard); 11 Wildcat (red) (guard); 5 civilian (civilian); 4 Wildcat (grey, AK-47) (guard); 3 Wildcat (rasta, pistol) (guard); 2 large woman (civilian); 2 Wildcat (red, shotgun) (guard); 2 woman (civilian); 2 Wildcat (red) (a body (dies at once)); 2 Wildcat (grey) (assassin); 2 cop (a body (dies at once)); 1 Wildcat (grey, AK-47) (gang member); 1 Wildcat (rasta) (sniper/assassin); 1 Wildcat (grey) (sniper/assassin); 1 workman (civilian); 1 Wildcat (rasta) (assassin).

</details>

**Objects:**

- Items: barrels x14, health pack x11, pistol ammo x6, shotgun ammo x2, grenade x2, knife
- Vehicles: van (locked) x3, sedan (locked), car (locked), taxi (unlocked), meat wagon (unlocked), police car (unlocked), jeep (unlocked), sedan (red key), Wildcat van (locked)
- Street furniture: bin x43, traffic cone x29
- Set pieces: 3 mist banks, 2 searchable props

**Badges:** 19. **Side objectives:**

- Arrest the Wildcat in the east (1 badge)
- Side objective (2 badges)
- Side objective (1 badge)
- Take down the Wildcat in the south (1 badge)
- Side objective (1 badge)

**Beacons (places and people the player is sent to):** "Wanted Felon", "Rear Access", "Boss Meeting", "Main Entrance", "Restaurant.", "Nightclub.", "Boss", "Armed Thug."

**How it ends:**

- **Lost** 7s later (after "Shit! What have I done? I'll never get t...").
- **Won** 3s later (after "I GOT HIM!! Now to get you back to the s...").

<details><summary>Script</summary>


*Conversation (on entering an area):*

> **Wildcat (Banes' runner):** ... Banes would kick my ass, maan. I AIN'T goin' back to him empty-handed, GET IT!  
> **Wildcat supplier:** Shit man, I already told ya - the shit will be there, don' worry!  
> **Wildcat (Banes' runner):** Don't forget those AK's neither! Last time, I had to...  


*Conversation (on entering an area):*

> **Gate Wildcat:** Beat it, stink ass! This here's PRIVATE PROPERTY for tonight - members of the Wildcat Brotherhood only!  
> **Darci:** UCPD don't need no invitations, chump. Step aside before you get hurt.  
> **Gate Wildcat:** Oooh, it won't be long before you and ALL UCPD will kneel at our FEET, ho!  
> **Darci:** Okay tough guy, you had your chance!  


*Conversation (when the player gets near):*

> **Georgie:** Sorry Officer - Wild Bill's told us not to let anybody in 'till the trouble dies down.  
> **Darci:** Trouble? That's why they called me. Y'know where I could find this "Wild Bill"?  
> **Georgie:** Sure - he's in the club at the moment - tell 'im Georgie sent ya.  
> **Darci:** Georgie, huh? Thanks...  

- **Darci:** Jeez, what's that smell? *(on entering an area)*
- **Civilian #167:** I'm going to see lusty Anna! *(on talking to Civilian #167)*
- **Civilian #166:** She's a saucy one. *(on talking to Civilian #166)*
- **Civilian #165:** (ahem)....evening, lady...uh, officer....uh,...miss... *(on talking to Civilian #165)*
- **Civilian #162:** Jeez - I hope that fat whore ain't dancin' again tonight! *(on talking to Civilian #162)*
- **Civilian #161:** Man, I can't wait for this place to open. *(on talking to Civilian #161)*
- **Tramp #175:** In you go lady! *(on talking to Tramp #175)*
- **Club-goer:** They say Wild Bill has been accepting BRIBES from the underworld - I don't buy it! *(on talking to Club-goer)*
- **Cop at the standoff:** Stern, that was great! *(on talking to Cop at the standoff)*
- **Cop at the standoff:** Well done Stern! *(on talking to Cop at the standoff)*
- **Civilian #230:** You ain't bought anything. *(on talking to Civilian #230)*
- **Civilian #226:** Hey, you want a beer? *(on talking to Civilian #226)*
- **Civilian #225:** Hello there, you pretty thing! *(on talking to Civilian #225)*
- **Civilian #221:** Oh maaan - the colors! *(on talking to Civilian #221)*
- **Civilian #222:** Hey there Darlin'! You wanna drink? I could.. uh.. Good day Officer! *(on talking to Civilian #222)*
- **Civilian #224:** SheeeIT! I should known - if the club is empty, it means it's a DUMP! Why'd I bother with this place? *(on talking to Civilian #224)*
- **Civilian #223:** Wild Bi..*hic!*..Bill? Who iss Wild Bill? I dunno, what you askin' m.. hic!.. me for? *(on talking to Civilian #223)*
- **Civilian #220:** Drink?? *(on talking to Civilian #220)*
- **Civilian #216:** Great tune this! *(on talking to Civilian #216)*
- **Civilian #218:** Man, I ain't goin' home tonight 'till all those Wildcat loonies are off the streets! *(on talking to Civilian #218)*
- **Civilian #215:** Well hello there young lassie! *(on talking to Civilian #215)*
- **Civilian #214:** The Wildcats got everyone here worried. Nobody jumps unless THEY say so. What the hell are the UCPD doin' about THAT? *(on talking to Civilian #214)*
- **Civilian #217:** Maan - She's a bit rough! No wonder this dump's empty. *(on talking to Civilian #217)*
- **Civilian #227:** Baby, baby! *(on talking to Civilian #227)*
- **Civilian #228:** Baby.. I luuv yoooou..baby..ooh! *(on talking to Civilian #228)*
- **Civilian #210:** Cha cha cha....! *(on talking to Civilian #210)*
- **Civilian #211:** Arright lady? How's about you an me, huh? *(on talking to Civilian #211)*
- **Civilian #212:** Cha cha cha! *(on talking to Civilian #212)*

*Conversation (on talking to Officer Mills):*

> **Officer Mills:** Hey Rookie. Got stuck on Traffic duty again! I hear there's some new gang in in town - how come I ain't out bustin' their heads?!  
> **Darci:** I hear ya, Officer Mills....  

- **Darci:** Control - Stern here. I  gotta tip about some Wildcats in the baseball park, I'm going to bag their asses. Stern out. *(at the start)*

*Conversation (when Darci is seen by Cop at the standoff):*

> **Cop at the standoff:** Hey Stern! BE careful down there, we got a situation!  
> **Darci:** Situation...??  
> **Cop at the standoff:** Two armed Wildcats. We got 'em flushed to a corner but they ain't comin' out!  
> **Darci:** Yeah? Maybe I can rouse 'em...!  

- **Cop at the standoff:** Hi Stern, any idea how we can flush out these Wildcat scum? *(on talking to Cop at the standoff)*
- **Cop at the standoff:** These guys don't wanna budge. We managed to get one of 'em but theres still two standing! Damn, if these assholes aren't the most determined scum we've come up against! *(on talking to Cop at the standoff)*

*Conversation (on entering an area):*

> **Wildcat with his girl:** What you want? I'm with my woman.  

- **Darci:** Oh man! looks like we got a body, an' it's been here some time too. *(on entering an area)*

*Conversation (after the conversation "What you want? I'm with my woman."):*

> **Wildcat's girlfriend:** Yeah, take a hike, dyke  

- **Despatch (radio):** Awwight Stern - but hurry it up will ya? We need you back on the streets ASAP! *(9s later (after "Control - Stern here. I  gotta tip about..."))*
- **Wildcat (Banes' runner):** The Law! Let's roll! *(when Darci is seen by Wildcat (Banes' runner))*
- **Darci:** Shit! What have I done? I'll never get that info for Roper now...!! *(when a count is reached)*
- **Despatch (radio):** Wild Bill? Talk to the guy by the balcony - he knows Bill. *(once both: after the conversation "Sorry Officer - Wild Bill's told us not ..."; and on talking to Civilian #219)*

*Conversation (on talking to Club doorman):*

> **Darci:** Hey there fella - guy called Georgie said I'd find Wild Bill in here.  
> **Club doorman:** Wild Bill? He's off limits, cop. He's being, uh.. protected.. by the Wildcats.  
> **Darci:** Wildcats? Listen asshole, I ain't got time to play around no more. You TELL me where Bill IS, or I'll haul you in!  
> **Club doorman:** Uh.. uh, okay lady, calm down, just.. stay cool now. I'll go phone Bill, tell him to expect you at Posheeta's Mexican Diner! Okay?  
> **Darci:** Fine. Now get to it.  

- **Despatch (radio):** Wild cat operatives arrested. *(once both: when Gate Wildcat is arrested; and when Gate Wildcat is arrested)*
- **Darci:** I GOT HIM!! Now to get you back to the station, boy. We got a hole put aside for you an' all your friends... *(when a count is reached)*

*Conversation (when Darci is seen by Tramp in his yard):*

> **Tramp in his yard:** HEY YOU! Stay away from my Goddamn car!  
> **Darci:** Uh...Sorry sir, just doing my job.  
> **Tramp in his yard:** Well go do your job some place else lady!  

- **Tramp in his yard:** I said LEAVE! This is MY yard! *(on talking to Tramp in his yard)*
- **Despatch (radio):** Jeez, Rookie - you topped the guy! *(when Tramp in his yard is killed (not arrested))*
- **Despatch (radio):** You'll find Bill at Posheetas down the road from here... *(after Wild Bill)*

*Conversation (on talking to Wild Bill):*

> **Darci:** 'Scuse me sir, are you "Wild Bill"?  
> **Wild Bill:** Bill is fine, young lady. I guess you were talkin' to my boys at the grounds then , huh?  
> **Darci:** Well...  
> **Wild Bill:** Yeah, I used to be the baseball team manager until last month. 'Till the goddamn Wildcats came along an' pushed me out! Anyway, how can I help ya pretty Lady?  
> **Darci:** I need to get into the baseball ground through that back alley.  
> **Wild Bill:** Ahh, Georgie told me there was some cop wantin' to get through. I'll just make the call to Georgie now...  
> **Darci:** Thanks "Bill".  


*Conversation (once both: on entering an area; and Wild Bill):*

> **Diner host:** Hey there miss - smoking or non-smoking?  
> **Darci:** Huh - no, no- I'm lookin' for some old guy called "Wild Bill"?  
> **Diner host:** Old Bill? He's the, uh, unkempt gentleman at the bar.  
> **Darci:** Thanks.  


*Conversation (when Darci is seen by Senior cop):*

> **Senior cop:** Dammit Rookie, takin' that old guy out was a bad move.. you're too damn free an' easy with your police powers!  
> **Darci:** C'mon, the guy was tryin' to kill me!  
> **Senior cop:** No buts girl, just think of the paperwork and next time, make a freakin arrest!  

- **Senior cop:** I can't say I'm happy with your attitude Rookie! You should save your aggression for the Wildcats - they been causing trouble ALL OVER Union City these past few days! *(on talking to Senior cop)*
- **Despatch (radio):** Okay lady. I gave the guys the go-ahead to let you in. I hope you cause those Wildcats ALL KINDS o' damage hyeh heh! *(3s later (when Wild Bill gets near))*

*Conversation (once either: earlier events; or earlier events):*

> **Hostage workman:** What do you want with us? Why can't we go back to work?  
> **Wildcat guard:** The Boss has a little.. business in the park. Until then, sit tight and shut up! Damn, I can't wait 'till we get rid of ALL of you non-believers.  


*Conversation (once both: on entering an area; and 12s later (after the conversation "HEY YOU! Stay away from my Goddamn car!")):*

> **Tramp in his yard:** THAT'S IT COPPER! I'M GONNA KICK YOUR ASS  

- **Despatch (radio):** Good Collar, Officer. *(once both: when Tramp in his yard is arrested; and after the conversation "THAT'S IT COPPER! I'M GONNA KICK YOUR AS...")*
- **Despatch (radio):** Guys, we got an APB: Armed and dangerous assailant, all units in district please attend. *(0s later (after "Okay lady. I gave the guys the go-ahead ..."))*
- **Despatch (radio):** Good Work Rookie - uh - Officer Stern! *(when Wildcat (red) #258 is down)*

</details>

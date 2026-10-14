# Rat Catch *(draft; `gang2`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\gang2.ucm`, map `gang1.iam`

**Played as:** Mako

**Story:** An earlier Rat Catch: fetch Bish and McLean, then 'Blood' and 'Kidnap'.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 10 | Wildcat contact | Wildcat (rasta) | civilian |  |
| 11 | Wildcat hanger-on | Wildcat (rasta) | civilian |  |
| 14 |  | civilian | civilian |  |
| 15 |  | civilian | civilian |  |
| 16 |  | civilian | civilian |  |
| 27 |  | civilian | civilian |  |
| 28 |  | civilian | civilian |  |
| 29 |  | civilian | civilian |  |
| 30 |  | civilian | civilian |  |
| 35 |  | civilian | civilian |  |
| 36 |  | civilian | civilian |  |
| 37 |  | civilian | civilian |  |
| 38 |  | civilian | civilian |  |
| 39 |  | civilian | civilian |  |
| 40 |  | civilian | civilian |  |
| 43 |  | civilian | civilian |  |
| 44 |  | civilian | civilian |  |
| 45 |  | civilian | civilian |  |
| 46 |  | civilian | civilian |  |
| 49 |  | civilian | civilian |  |
| 52 |  | civilian | civilian |  |
| 53 |  | civilian | civilian |  |
| 110 |  | Wildcat (rasta) | civilian |  |
| 113 |  | Wildcat (rasta) | civilian |  |
| 120 | Spiv | Wildcat (rasta) | civilian |  |
| 122 | McIntyre's son | civilian | civilian |  |

Others: 20 civilian (civilian); 17 civilian (driver); 1 cop (driver).

</details>

**Objects:**

- Items: barrels x10
- Vehicles: car (unlocked) x11, taxi (unlocked) x5, police car (unlocked) x2

**Beacons (places and people the player is sent to):** "Mclean", "Bish", "Blood", "Kidnap"

<details><summary>Script</summary>


*Conversation (on talking to Wildcat contact):*

> **Wildcat contact:** Go fetch hoods Bish and Mclean, then you be told what to do next.  
> **Mako:** Okay but where are they  
> **Wildcat contact:** Check your nav map.  

- **Wildcat hanger-on:** Go get Bish and Mclean! *(on talking to Wildcat hanger-on)*
- **Civilian #37:** Hey, don't hurt me. *(on talking to Civilian #37)*
- **Civilian #52:** Please don't hurt me. *(on talking to Civilian #52)*
- **Civilian #15:** People like you are scum. *(on talking to Civilian #15)*
- **Civilian #43:** Get lost.! *(on talking to Civilian #43)*
- **Civilian #16:** Get the hell away from me! *(on talking to Civilian #16)*
- **Civilian #40:** Leave me alone! *(on talking to Civilian #40)*
- **Civilian #39:** I'll call the cops if you don't get lost! *(on talking to Civilian #39)*
- **Civilian #38:** Get outta here you thug! *(on talking to Civilian #38)*
- **Civilian #53:** More like pussy cats than Wild cats! *(on talking to Civilian #53)*
- **Civilian #14:** Oh..please don't hurt me. *(on talking to Civilian #14)*
- **Civilian #44:** Help! *(on talking to Civilian #44)*
- **Civilian #45:** You look hard as hell...I'm a bit scared! *(on talking to Civilian #45)*
- **Civilian #46:** Aaaaargh!! *(on talking to Civilian #46)*
- **Civilian #49:** Do I know you? *(on talking to Civilian #49)*
- **Civilian #30:** Gee, your ugly! *(on talking to Civilian #30)*
- **Civilian #27:** Nice haircut dude. *(on talking to Civilian #27)*
- **Civilian #29:** Freak! *(on talking to Civilian #29)*
- **Civilian #28:** People like you should be put away!! *(on talking to Civilian #28)*
- **Civilian #36:** Get outta here.! *(on talking to Civilian #36)*
- **Civilian #35:** Your scum! *(on talking to Civilian #35)*

*Conversation (on talking to Wildcat (rasta) #110):*

> **Wildcat (rasta) #110:** Hi..I'm Bish..wassup?  
> **Mako:** Da boss wants you to come with me, we got some business.  
> **Wildcat (rasta) #110:** Okay!  


*Conversation (on talking to Wildcat (rasta) #113):*

> **Wildcat (rasta) #113:** Hiya man!  
> **Mako:** You need to come with me...we've got work to do.  
> **Wildcat (rasta) #113:** Er...yeah okay man.  

- **Despatch (radio):** Okay you guys, listen up. Get to new nav point to meet Blood for further instructions. *(once both: after the conversation "Hiya man!"; and after the conversation "Hi..I'm Bish..wassup?")*

*Conversation (on talking to Spiv):*

> **Mako:** Yo Blood!  
> **Spiv:** Hey man, nice ta see ya..  
> **Mako:** You got us some work?  
> **Spiv:** Yeah, you'll like this.  
> **Mako:** ?  
> **Spiv:** Follow your next nav pointer and kidnap a Mr Frazer Mcintyre...  
> **Mako:** Whos he?  
> **Spiv:** He's the father of that cops friend. He will be very useful as a bargaining tool.  
> **Mako:** Spare us the deatils, just show us the way.  
> **Spiv:** Thats the attitude lads.  


*Conversation (on talking to McIntyre's son):*

> **McIntyre's son:** Woooa..who are you guys?  
> **Mako:** Just come with us and we'll make it easy for ya!  
> **McIntyre's son:** Hey now come on...what do you want from me?  
> **Mako:** Just shut it and walk!  
> **McIntyre's son:** You'll have to fight me first...  

- **Despatch (radio):** Oh no...you killed the guy...get back to the den...! *(when McIntyre's son is down)*
- **Despatch (radio):** Well done...that guy was just too dumb. *(once both: when the player gets near; and "Oh no...you killed the guy...get back to...")*

</details>

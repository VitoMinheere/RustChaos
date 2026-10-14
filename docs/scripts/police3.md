# Gun Hunt

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\police3.ucm`, map `disturb1.iam`, district West District

**Played as:** Darci

**Briefing:**

> An honest citizen has informed us that he saw a man dumping a firearm at the back of the Eagle Building. He can't remember the exact spot so you'll need to search the area for the weapon. This weapon may be linked to a recent murder, so it is imperative that you bring it back to the station once found.

**Story:** An honest citizen saw a man dump a gun behind the Eagle Building, and Deeks knows where. Darci finds him in the park; he points her to the skips in the centre of town. Once she has the weapon, its armed owner comes looking for it, and she must arrest or drop him before taking the gun back to the station. Meanwhile the city is uneasy: a drunk tramp warns of 'dark things on the horizon', people gossip that there's 'something of the night about that Bane character', and a cop says one Wildcat set was caught with a vanload of AK-47s. A disturbance at the diner, where men call the customers SINNERS, gives two more suspects. Killing or arresting Deeks fails it.

**Characters:**

- **Darci Stern**
- **Deeks**: the snitch
- **Sergeant Bryant** (mentioned)
- **The armed suspect**: the gun's owner
- **Two diner suspects**: Wildcats calling people sinners (guilty)
- A drunk tramp, a man who has to 'eat ALL the pigeons', a man with a balloon, a gossiping cop, Bane-wary citizens

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 38 | Pigeon man | civilian | civilian |  |
| 51 | Diner witness | civilian | civilian |  |
| 62 | Drunk tramp | tramp | civilian |  |
| 69 | Generous passer-by | civilian | civilian |  |
| 74 | Armed suspect | Wildcat (rasta, pistol) | gang member |  |
| 103 |  | Wildcat (red) | guard |  |
| 104 |  | Wildcat (red) | guard |  |
| 114 |  | cop | driver |  |
| 124 | Diner suspect 1 | Wildcat (grey) | civilian | guilty, stands still |
| 125 | Diner suspect 2 | Wildcat (grey) | bully | guilty, stands still |
| 148 | Passer-by | civilian | civilian |  |
| 151 | Passer-by | civilian | civilian |  |
| 152 | Man with a balloon | civilian with a balloon | civilian |  |
| 153 | Deeks | tramp | civilian |  |
| 167 | Woman asking about the Wildcats | woman | civilian |  |
| 169 | Gossiping cop | cop | cop |  |
| 182 | Bane-wary citizen | civilian | civilian |  |
| 184 | Doom-sayer | civilian | civilian |  |
| 195 | Woman | woman | civilian |  |

Others: 4 civilian (civilian); 2 Wildcat (grey) (guard); 2 Wildcat (rasta, pistol) (guard); 2 Wildcat (red) (guard); 1 cop (cop); 1 woman (guard); 1 cop (driver); 1 civilian (driver).

</details>

**Objects:**

- Items: health pack x3, pistol
- Vehicles: police car (unlocked) x4, car (unlocked)
- Street furniture: bin x15

**Badges:** 20. **Side objectives:**

- Side objective (1 badge)
- Take down the Wildcat in the south-west (3 badges)
- Side objective (1 badge)

**Beacons (places and people the player is sent to):** "District Station", "Suspected Firearm", "Armed suspect", "Diner Suspect 1", "Diner Suspect 2.", "Park: Find Deeks", "Station"

**How it ends:**

- **Lost** once either: "SHIT! Deeks is dead!"; or "You arrested Deeks!".
- **Won** after "Excellent work officer. You have exceede...".

<details><summary>Script</summary>

- **Darci:** Deeks has gotta be someplace near here! *(on entering an area)*
- **Street sign:** Zarb Drive *(on entering an area)*
- **Street sign:** Keating Ave *(on entering an area)*
- **Despatch (radio):** D'arci, that snitch Deeks has some news of a stashed weapon in this locality. We need that weapon as evidence, D'arci. Deeks is in the park - go to it. *(at the start)*
- **Despatch (radio):** SHIT! Deeks is dead! *(when Deeks is killed (not arrested))*
- **Despatch (radio):** You arrested Deeks! *(when Deeks is arrested)*
- **Generous passer-by:** Busy tonight, huh? Have this. *(on talking to Generous passer-by)*
- **Diner witness:** Officer, there's some real shady characters up at the Diner. They were shouting at the customers, callin' them all SINNERS! *(on talking to Diner witness)*

*Conversation (on talking to Pigeon man):*

> **Darci:** Wooa! Slow down there buddy.  
> **Pigeon man:** Sorry miss, can't stop - gotta eat ALL the pigeons!  


*Conversation (on talking to Deeks):*

> **Deeks:** Psst - Hey Stern! It's me.. Deeks.  
> **Darci:** Deeks? Sgt. Bryant said you had some info on that weapon?  
> **Deeks:** Yeah - It's behind some skips in the center of town.  
> **Darci:** Gotcha - thanks Deeks.  
> **Deeks:** See ya, Stern!  

- **Woman asking about the Wildcats:** So what's this I hear about some new gang called the Wildcats? You brought any of 'em in yet? *(on talking to Woman asking about the Wildcats)*

*Conversation (on talking to Gossiping cop):*

> **Gossiping cop:** Hi Darci. Shouldn't you be on duty?  
> **Darci:** Yeah I need to find some murder weapon.  
> **Gossiping cop:** You hear about what these Wildcats been doin'?  
> **Darci:** Yeah a little...  
> **Gossiping cop:** Bryant says he heard that one Wildcats set was caught with a van load of AK47s!  
> **Darci:** AKs? Hell, they must think they're a MILITIA now...  

- **Drunk tramp:** (HIC).. Girl, there's dark things.. (HIC).. on the horizon for Union shhity. Get out while you (hic) can! *(on entering an area)*
- **Passer-by:** You seen that loony? *(on talking to Passer-by)*
- **Doom-sayer:** We're gonna all die! I know it...I KNOW it! *(on talking to Doom-sayer)*
- **Woman:** Leave it out girl! *(on talking to Woman)*
- **Bane-wary citizen:** I tell ya, there's something of the night about that Bane character. *(on talking to Bane-wary citizen)*
- **Man with a balloon:** Deeks? Well I ain't him! *(on talking to Man with a balloon)*
- **Passer-by:** You seen the park nut-case? He's a gas! *(on talking to Passer-by)*

*Conversation (on talking to Drunk tramp):*

> **Drunk tramp:** Euugh.. I don' fee too (HIC!) good..  
> **Darci:** Allright, I think you'd better come with me sir..  
> **Drunk tramp:** Unngh.. I shouldn't be (HIC!) drinkin' this bottle o' medicine.. here, you have it.. uuh.  


*Conversation (on picking up the pistol #54):*

> **Darci:** Control? I got that weapon secured  

- **Despatch (radio):** All units - we got a description of an armed man in your area. Arrest or eliminate. *(10s later (on picking up the pistol #54))*
- **Despatch (radio):** Good work Rookie. Continue your patrol, or return to the station now. Any sign o' those Wildcat crazies? *(after the conversation "Control? I got that weapon secured")*
- **Despatch (radio):** Situation under control. Rookie, get that gun back here. *(when Armed suspect is down)*
- **Despatch (radio):** Come in Officer Stern - we got reports of a disturbance over at the diner. Get on it ASAP. *(70s later (earlier events))*
- **Despatch (radio):** Excellent work officer. You have exceeded requirements. *(once both: on entering an area; and "Situation under control. Rookie, get tha...")*

</details>

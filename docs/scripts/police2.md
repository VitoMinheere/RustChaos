# The Jump

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\police2.ucm`, map `disturb1.iam`, district West District

**Played as:** Darci

**Briefing:**

> Stern I gotta tell you, today's been a weird day. Get this - there's a JUMPER hanging off a building over on Keating Ave. Get over there and get him down - but D'arci, do it subtle. O'Byrne says the guy's as nervous as a turkey at Thanksgiving, won't let anyone approach him directly. We need this kook taken out of trouble. If the press gets hold of this they'll hang our asses out to dry.

**Story:** A jumper is on a ledge of Brooks Tower on Keating Avenue. He's Barnaby, depressed because the Union City Fire Department turned him down, and he jumps if anyone approaches him head-on. Darci climbs the next building and rides the zip line down behind him, then talks him off: he decides he'd rather be a cop. Along the way: officer Miles' squad car has been stolen (a streetwalker saw a 'slick dude' on Lime Street, a Wildcat car thief), and a crazed woman in the park turns out to be 'Crack Suckin' Susan'. A wounded woman lies at a shooting where a punk screamed 'Union City will fall!', the first sign of the Wildcats' war on the city.

**Characters:**

- **Darci Stern**
- **Barnaby**: the jumper, wants to be a fireman, then a cop
- **Desk sergeant**: relays officer Liebnitz's report
- **O'Byrne**, **Liebnitz**, **Officer Frank** (mentioned)
- **Officer Miles**: lost his squad car
- **Old cop at the tower**: retiring next month
- **Streetwalker** who saw the car thief
- **Car thief**: Wildcat on Lime Street
- **'Crack Suckin' Susan'**
- **Rooftop Wildcat**: 'we gonna TAKE this city'
- Onlookers betting on blood; three women in a developer joke ('Mucky test')

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 9 | Cop | cop | cop |  |
| 14 | The 'Player' man | civilian | civilian |  |
| 21 | Officer Miles | cop | cop | stands still |
| 31 | Mucky woman | woman | civilian | invulnerable, stands still |
| 32 | Mucky woman | woman | civilian | invulnerable, stands still |
| 33 | Mucky woman | woman | civilian | invulnerable, stands still |
| 54 | Barnaby | civilian | stands about | stands still |
| 55 | Officer Miles | cop | civilian | invulnerable, stands still |
| 69 | Passer-by | civilian | civilian |  |
| 80 | Onlooker | civilian | civilian |  |
| 86 | Onlooker | civilian | civilian |  |
| 87 | Streetwalker | large woman | civilian | stands still |
| 88 | Onlooker | civilian | civilian |  |
| 89 | Onlooker | civilian | civilian |  |
| 92 | Onlooker | civilian | civilian |  |
| 93 | Onlooker | civilian | civilian |  |
| 94 | Onlooker | civilian | civilian |  |
| 124 | Car thief (Wildcat) | Wildcat (grey) | stands about | guilty, stands still |
| 208 | Cop | cop | cop |  |
| 216 | Old cop at the tower | cop | civilian |  |
| 218 | Cop on the stairs | cop | cop |  |
| 242 | Cop at the shooting | cop (pistol) | cop |  |
| 243 | Cop | cop (pistol) | cop |  |
| 249 | Large woman | large woman | civilian |  |
| 251 | 'Crack Suckin' Susan' | woman | guard |  |
| 255 | Desk sergeant | cop (pistol) | cop |  |
| 272 | Park bench man | civilian | civilian |  |
| 274 | Workman | civilian | civilian |  |
| 280 | Rooftop Wildcat | Wildcat (grey) | guard |  |

Others: 4 cop (cop driver); 2 Wildcat (red) (guard); 1 cop (pistol) (cop); 1 civilian (driver); 1 woman (a body (dies at once)).

</details>

**Objects:**

- Items: baseball bat, health pack
- Vehicles: police car (locked) x8, police car (unlocked) x5, ambulance (locked), ambulance (unlocked)
- Street furniture: bin x13, traffic cone x6

**Badges:** 19. **Side objectives:**

- Talk to the cop near Lime Street (2 badges)
- Talk to the woman in the north (4 badges)
- Take down the woman near Kleetus Ave (1 badge)

**Beacons (places and people the player is sent to):** "District Station", "Brooks Tower", "Lime Street", "Kleetus Avenue.", "Police Station"

**How it ends:**

- **Won** after "Arright return to the Station D'arci. An...".
- **Lost** 2s later (after "Shit Stern - thanks a bunch! Next time t...").

<details><summary>Script</summary>

- **Street sign:** Kleetus Ave. *(on entering an area)*
- **Street sign:** Lime Street *(on entering an area)*

*Conversation (on entering an area):*

> **'Crack Suckin' Susan':** What you want?  
> **Darci:** Okay miss, get movin'  
> **'Crack Suckin' Susan':** Don't you.. DON'T YOU TOUCH ME!  
> **Darci:** Huh? She's nuts!  


*Conversation (at the start):*

> **Desk sergeant:** 'Kay Rookie, Officer Liebnitz just called in from the scene - says the jumper's crazy as a loon!  
> **Darci:** HUH! Well, Liebnitz was always perceptive!  
> **Desk sergeant:** Apparently the guy's depressed because.. oh Lord.. because he's been turned down by the UC Fire Dept!  
> **Darci:** WHAT??  
> **Desk sergeant:** Hey, that's what Liebnitz's reports says!  
> **Darci:** Yeah? Well does it say anything  that's actually useful?  
> **Desk sergeant:** Well, apparently as soon as he sees anybody within 6 feet of him, he threatens to go over..  
> **Darci:** As soon as he SEES anybody, huh? Guess I'll have to take the quiet approach.  
> **Desk sergeant:** Good luck, Stern - now roll out.  

- **The 'Player' man:** The aliens that control my destiny told me - "give this to the player!" Are you the "Player"? *(on talking to The 'Player' man)*
- **Despatch (radio):** Mucky test revealed. *(once both: on pressing action here; and on pressing action here)*
- **Passer-by:** Hi officer, nice to see you again. *(on talking to Passer-by)*
- **Cop:** C'mon Rookie, get yer ass up there. *(on talking to Cop)*
- **Onlooker:** I heard there was a bad accident here. I ain't never seen a REAL crash.. was there any blood Officer? Any brains on the windscreen? Huh? *(on talking to Onlooker)*
- **Onlooker:** The guys gone Buggo! Maaan, we're gonna see some blood TONIGHT! *(on talking to Onlooker)*
- **Onlooker:** Someone should help him. *(on talking to Onlooker)*
- **Onlooker:** DAMMIT! WHERE'S MY CAMERA??!!?? *(on talking to Onlooker)*
- **Onlooker:** Jeez - it's a long way down.... *(on talking to Onlooker)*
- **Onlooker:** What a maroon. HEY BUDDY! YOU CAN FLY -  WHY DON'T YA COME ON DOWN? *(on talking to Onlooker)*
- **Onlooker:** He must have a good view.. uh huh. *(on talking to Onlooker)*
- **Cop:** Hey there Stern. I think O'Doole's coming to pick up the cones. *(on talking to Cop)*

*Conversation (on talking to Old cop at the tower):*

> **Old cop at the tower:** Hey Stern. The chumps up there.. name's "Barnaby"  

- **Cop on the stairs:** Officer Frank is near the top of Brooks Tower. Its up this way. *(on talking to Cop on the stairs)*

*Conversation (when the player gets near):*

> **Darci:** Hey, hey man - what's your name?  
> **Barnaby:** M-me? Barnaby!  
> **Darci:** Bar..? Well "Barnaby", what say you come down from that ledge?  
> **Barnaby:** All I wanted was to be a Fireman! Why can't I be a Fireman?  
> **Darci:** Hey - you could, you could, uh.. try another Fire Department in another city!  
> **Barnaby:** No I wanna be a UNION CITY Fireman! They've got the best uniforms...  
> **Darci:** Good grief!  
> **Barnaby:** ..the shiniest poles..  
> **Darci:** Hey! Come back from there!  
> **Barnaby:** ...and the BIGGEST, REDDEST FIRE TRUCKS KNOWN TO MAN!  
> **Darci:** Wait NO!  
> **Barnaby:** ALL I WANTED WAS A HOSE OF MY OWN..  


*Conversation (on talking to Cop at the shooting):*

> **Darci:** What the hell..?  
> **Cop at the shooting:** Looks like she took one in the stomach - nasty.  
> **Darci:** Aw Jeez....anyone see the assailant?  
> **Cop at the shooting:** No.. but a guy says he heard some young punk screaming "Union City will fall!".  

- **Cop:** This is senseless - we've gotta bust those Wildcat punks before they get too uppity! *(on talking to Cop)*
- **Large woman:** What the hell do you want? *(on talking to Large woman)*
- **Darci:** Dispatch? I just caught some CRAZY woman in the park...!! *(when 'Crack Suckin' Susan' is down)*
- **Park bench man:** Uh... am I allowed to sit here? *(on talking to Park bench man)*
- **Workman:** Yeah, you cops would talk all day if ya could. Not like us workin' stiffs! *(on talking to Workman)*

*Conversation (when Darci is seen by Rooftop Wildcat):*

> **Rooftop Wildcat:** Yay, pig. What you doin' up here?  
> **Darci:** Lookin' for a fight....  
> **Rooftop Wildcat:** Oh, Wildcats FIGHT all right....we gonna TAKE this city - you watch!  
> **Darci:** I'm gonna take YOU, punk..  

- **the bin #57:** FIREAAAAAAAAAH! *(when Barnaby gets near)*
- **Despatch (radio):** Talk to the muckiest. *(once both: on entering an area; and "Mucky test revealed.")*
- **Despatch (radio):** Shit Stern - thanks a bunch! Next time there's a jumper needing subtley, maybe I'll just pin a badge on a DAMN ELEPHANT! Get back here now! *(when Barnaby is killed (not arrested))*

*Conversation (3s later (after "Someone should help him.")):*

> **Onlooker:** Help him? Yeah, they should help him OFF THE SIDE!  
> **Onlooker:** Ain't you got no empathy pal?  
> **Onlooker:** Aaaah, empathy my ass. The guy's gone wacko - let him take a dive, I say!  

- **Despatch (radio):** Good work Stern - That'll be "Crack Suckin' Susan", she's always plying her trade down the park way. *(after "Dispatch? I just caught some CRAZY woman...")*
- **Despatch (radio):** D'arci! We got a distress call in from Officer Miles. Meet him at Kleetus Avenue ASAP. *(60s later (on entering an area))*

*Conversation (once either: on entering an area; or on entering an area):*

> **Darci:** Awwight pal, let's go  
> **Barnaby:** HUH? Where the hell did YOU come from?  
> **Darci:** The sky - now let's take this niiice an' easy  
> **Barnaby:** But.. I wanna be a.. fireman..  
> **Darci:** Lissen pal, if I come down from this building empty handed, I'm gonna get it in the ass-so DON'T PUSH ME!  
> **Barnaby:** Oh no, I might hurt you this high up.. But I'm staying put Officer...  
> **Darci:** Now, wait a....  
> **Barnaby:** ...officer...Officer??...Officer BARNABY! I WANNA BE A COP!  
> **Darci:** Yeah, well I'll put in your application personally!  
> **Barnaby:** You will?  
> **Darci:** Sure - follow me  
> **Barnaby:** Right-oh!  


*Conversation (after the conversation "Hey Stern. The chumps up there.. name's..."):*

> **Barnaby:** HEY YOU DOWN THERE! DON"T THINK I CAN'T HEAR YOU TALKNG ABOUT ME!  
> **Old cop at the tower:** ...oh Maaan...!  
> **Barnaby:** IF YOU COME UP, I'LL JUMP!  
> **Old cop at the tower:** AHH SHADDAP FIREBOY!  

- **Mucky woman:** Well done - it's me! *(on talking to Mucky woman)*
- **Mucky woman:** Sorry, not me neither. *(on talking to Mucky woman)*
- **Mucky woman:** Not me lady! *(on talking to Mucky woman)*

*Conversation (on talking to Officer Miles):*

> **Officer Miles:** Stern - thank GOD you're here. I gotta BIG problem!  
> **Darci:** Okay Miles - shoot  
> **Officer Miles:** Well, it's kinda embarrassing.. don't say this to the guys or nothin'...  
> **Darci:** This isn't a MAN thing, is it? 'Cos if so, I've gotta run...  
> **Officer Miles:** NO, no, it's a...uh, cop thing...  
> **Darci:** Yeah?  
> **Officer Miles:** Y'see I was workin' the crowd at the jumpers building and.. an' my squad car got.. uh.. stolen  
> **Darci:** Jeez, Miles. I'm a damn ROOKIE an' even I gotta laugh at THAT!  
> **Officer Miles:** Yeah, but.. can you help me out? I gotta get back to the Jumper soon, I ain't got time to go walkabouts  
> **Darci:** Okay Miles, I'll have a look-see. You got ANY clues AT ALL?  
> **Officer Miles:** Well, there was a streetwalker close at the time, she might've seen somethin'..  
> **Darci:** Stay here an' I'll get back to you.  
> **Officer Miles:** Thanks, Stern.  

- **Despatch (radio):** Arright return to the Station D'arci. And well played - seems like the guy appreciated your "womans touch"...HAW HAW *(after the conversation "Awwight pal, let's go")*

*Conversation (after the conversation "HEY YOU DOWN THERE! DON"T THINK I CAN'T ..."):*

> **Old cop at the tower:** You see what I gotta deal with? Thank God I retire next month!  

- **Darci:** Jerk. As if rookie's ain't got better things to do than look for his damn car.. Miles is such a screw-up! *(after the conversation "Stern - thank GOD you're here. I gotta B...")*
- **Barnaby:** I heard that asshole! *(3s later (after "What a maroon. HEY BUDDY! YOU CAN FLY - ..."))*
- **Darci:** Asshole. I get back to the station, I'm gonna use my "womans touch" to break his damn legs. *(4s later (after "Arright return to the Station D'arci. An..."))*

*Conversation (on talking to Streetwalker):*

> **Darci:** S'cuse me Ma'am  
> **Streetwalker:** Yeah Honey.. damn, a cop! Just my luck to get busted just as I'm about to turn in.  
> **Darci:** Wooah there, I ain't interested in you today, Miss - I need some information  
> **Streetwalker:** Uh-huh...Don't you cops always?  
> **Darci:** There was a squad car stolen from a, uh, UCPD officer a while back - did you see anythin'?  
> **Streetwalker:** Alright, seeing as your lettin' me go...I saw some slick dude up top of Lime street - he's your man.  
> **Darci:** Thanks - you've been a big help lady  
> **Streetwalker:** Yeah, an' then you'll STILL come back an' bust me tomorrow. GEDOUDAHEAH!  


*Conversation (when Darci is seen by Car thief (Wildcat)):*

> **Car thief (Wildcat):** What you want, Whore?  
> **Darci:** Hmm, so you're the jerk that thinks stealin' squad cars is fun, huh?  
> **Car thief (Wildcat):** You don' get it, Cop - your time in Union City is OVER!  
> **Darci:** Damn! How come all you Wildcats is so STUPID?  
> **Car thief (Wildcat):** Don't mess with me, woman.  
> **Darci:** Try me, asshole.  

- **Despatch (radio):** Hmm.. I wonder if this is Miles'...? *(once both: once both: earlier events; and "Stern - thank GOD you're here. I gotta B..."; and the police car #13)*

*Conversation (when Darci is seen by Officer Miles):*

> **Officer Miles:** Shit Stern you found my car! Thank God for rookies! Hey, I've left you a small gift around the corner!  


</details>

# Estate of Emergency

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\estate2.ucm`, map `nestate01.iam`, district Banes Estate

**Played as:** Darci, then Darci, Roper

**Briefing:**

> Mack Bane and his flunkies are making their retreat, D'arci. Word has it he's fled back to his out-of-town estate before making for distant lands. That's where we'll take him down. One thing - we need him alive. You've got to get in there, beat his ass down and drag him back to the station. We need this devil in a deep, murky dungeon for the next few weeks - and a UCPD jail cell is the nearest thing to a dark pit that I've ever seen. Be aware that the Bane estate is protected by a sonic alarm system, so you've got to shut this security system down before making your attempt!

**Story:** Bane is retreating to his out-of-town estate before fleeing abroad, and the UCPD needs him alive. Darci gets past the guards, Roper (now openly sent by Washington at the Chief's request) saves her life, and she tells him she overheard Wildcats talking about MISSILES aimed at Union City. Inside, Bane speaks to her: he knows her and 'the old man, Roper' and has known them both 'for almost a thousand years'. She has to take the security system offline from the pool house and arrest Bane before his getaway helicopter is refuelled. Killing him, or letting him fly, fails it.

**Characters:**

- **Darci Stern**, with **Roper**
- **Mack Bane**
- **Bane's pilot**
- Wildcat guards, cops at the perimeter

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 41 | Bane | tramp | civilian | invulnerable, stands still |
| 47 | Wildcat | Wildcat (red) | civilian | stands still |
| 95 | Cop | cop (pistol) | cop |  |
| 109 | Roper | Roper | stands about |  |
| 172 | Bane | tramp | runs from the player | guilty |
| 173 | Bane's pilot | workman | civilian | guilty, stands still |
| 230 | Wildcat | Wildcat (red) | civilian | stands still |
| 293 | Wildcat | Wildcat (red) | civilian | stands still |

Others: 44 Wildcat (grey) (guard); 25 Wildcat (rasta) (guard); 17 Wildcat (red) (guard); 10 cop (pistol) (cop); 7 Wildcat (red) (assassin); 6 Wildcat (grey) (assassin); 5 Wildcat (rasta, pistol) (guard); 5 Wildcat (red) (a body (dies at once)); 3 Wildcat (rasta) (sniper/assassin); 2 Wildcat (grey, AK-47) (guard); 2 Wildcat (rasta) (assassin); 1 Wildcat (grey, AK-47) (assassin); 1 cop (pistol) (a body (dies at once)); 1 Man In Black (Man In Black); 1 civilian (driver).

</details>

**Objects:**

- Items: health pack x6, shotgun ammo x5, baseball bat x2, grenade, pistol ammo
- Vehicles: car (locked) x13, van (locked) x8, police car (locked) x3, Wildcat van (locked) x3, police helicopter (locked), Wildcat van (unlocked)
- Street furniture: bin x17
- Set pieces: 1 steam traps

**Badges:** 14

**Beacons (places and people the player is sent to):** "Main Building", "Override switch.", "Darci", "Getaway Chopper.", "Bane"

**How it ends:**

- **Lost** after "Shit Bane is dead! We needed him ALIVE!".
- **Won** 2s later (when Bane is arrested).
- **Lost** after "Bane got away DAMMIT!".

<details><summary>Script</summary>

- **Despatch (radio):** Security systems off! *(on pressing action here)*
- **Despatch (radio):** INTRUDER ALERT! *(on entering an area)*
- **Despatch (radio):** Bane is hiding within this building. To get in, you'll need to take the security system off-line! *(at the start)*

*Conversation (at the start):*

> **Cop:** So how come the Captain sent YOU down here?  
> **Roper:** Your Chief asked Washington for help controlling the riots and Gang war in Union City, and I'm it.  
> **Cop:** YOU? What about the damn National Guard fer cryin' out loud? Or the SWAT teams? Where the hell IS everybody?  


*Conversation (when Darci gets near):*

> **Bane:** So, they send in their WOMEN to catch the bad guys now, HAH?  
> **Darci:** This woman just carved up your top men, Bane.  
> **Bane:** Hunh.. that you did, D'arci Stern.  
> **Darci:** You know me?  
> **Bane:** "From Caledonia and ancient Nubia.. " Yes Stern, I know you.. and the old man, Roper. I've known you both for.. almost a thousand years!  

- **Despatch (radio):** The security systems can be disarmed by throwing the override switch in the pool house. *(at the start)*

*Conversation (once both: once both: earlier events; and earlier events; and when Wildcat is down):*

> **Darci:** Thanks Roper.. looks like I owe you my life again!  
> **Roper:** Hell, D'arci, I can't let you die.. not before all the fun begins!  
> **Darci:** What fun? The city's being taken over! The cops are all off huntin' murderers and kidknappings - there ain't nobody left except us!  
> **Roper:** Let's hope that old man Roper and rookie Stern are enough then, hunh?  
> **Darci:** Roper wait.. this thing has gotten worse. I overheard some Wildcats talkin' about MISSILES, Roper - missiles aimed at Union City!  


*Conversation (after the conversation "Thanks Roper.. looks like I owe you my l..."):*

> **Bane:** How long until this thing will be refuelled?  
> **Bane's pilot:** About 5 minutes sir!  
> **Bane:** I'll give you THREE!  

- **Despatch (radio):** Shit Bane is dead! We needed him ALIVE! *(when Bane is killed (not arrested))*
- **Despatch (radio):** Bane got away DAMMIT! *(once both: when Bane is seen by Darci; and on a 180s countdown (after the conversation "How long until this thing will be refuel..."))*

</details>

# El Cossa Island *(unofficial title; `jung3`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\jung3.ucm`, map `jung3.iam`, district El Cossa Island (in the district list, unused by any shipped mission)

**Played as:** Darci

**Story:** The jungle mission, a bonus meant to follow the main story (it comes right after the 33 in the frontend's recommended order, has its own textures and speech, and is mission 31 in the console conversion table). After Gaemin Research's silos, Bane and his men have retreated to El Cosso Island, where a construction site may be building MORE silos. Darci lands on the jetty, makes her way into the Launcher Compound past steam vents and mines, and takes down Bane's fanatical workmen. Island residents have already been killed; she has to keep the rest of the hostages alive. Picking up a certain AK-47 earns 'Lord of the Jungle!'. Note the spelling: the district list says El Cossa, Despatch says El Cosso.

**Characters:**

- **Darci Stern**
- **Bane's workmen**: fanatical followers building the silos
- **Island residents** held hostage
- A frightened man at the jetty, Wildcats, a Man In Black

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 2 | Man at the jetty | civilian | civilian | stands still |
| 57 | Bane workman | workman | guard |  |
| 58 | Bane workman | workman | guard |  |
| 59 | Bane workman | workman | guard |  |
| 60 | Bane workman | workman | guard |  |
| 61 | Bane workman | workman | guard |  |
| 64 | Bane workman | workman | guard |  |
| 81 | Islander | civilian | civilian | stands still |
| 82 | Islander | civilian | civilian | stands still |
| 93 | Islander | civilian | civilian | stands still |
| 94 | Islander | civilian | civilian | stands still |

Others: 9 Wildcat (rasta, pistol) (guard); 7 civilian (a body (dies at once)); 6 Wildcat (red) (guard); 3 Wildcat (grey) (guard); 2 Wildcat (rasta) (assassin); 2 Wildcat (rasta) (guard); 1 Man In Black (Man In Black); 1 Wildcat (grey) (assassin).

</details>

**Objects:**

- Items: barrels x21, mine x11, AK-47 ammo x3, pistol ammo x2, health pack x2, grenade, AK-47
- Vehicles: van (unlocked)
- Set pieces: 4 steam traps

**Badges:** 13

**Beacons (places and people the player is sent to):** "Launcher Compound", "Target"

**How it ends:**

- **Lost** once either: "You killed a hostage!"; or "You killed a hostage!".
- **Lost** once either: "You killed a hostage!"; or "You killed a hostage!".
- **Lost** after "You killed one of the hostages!".
- **Won** after "All Bane workers down!".

<details><summary>Script</summary>


*Conversation (when the player gets near):*

> **Man at the jetty:** Wait maan, don't shoot me!!  
> **Darci:** Where's the armed men? Where did they go?  
> **Man at the jetty:** I didn't see, I was duckin' for cover as soon as the shootin' started!  

- **Despatch (radio):** All Bane workers down! *(when the whole group is down)*
- **Despatch (radio):** Gate Open. *(on pressing action here)*
- **Bane workman:** GUYS, ITS THAT COP! Murderize her! *(when Darci is seen by Bane workman)*
- **Despatch (radio):** You killed a hostage! *(when Islander is killed (not arrested))*
- **Despatch (radio):** You killed a hostage! *(when Islander is killed (not arrested))*
- **Despatch (radio):** You killed a hostage! *(when Islander is killed (not arrested))*
- **Despatch (radio):** You killed a hostage! *(when Islander is killed (not arrested))*
- **Despatch (radio):** You killed one of the hostages! *(when Man at the jetty is killed (not arrested))*
- **Despatch (radio):** That nut Bane an' his men have retreated to El Cosso Island. We got sightings of a construction site, possibly building MORE silo's. We want those sites LEVELLED, Stern! *(at the start)*
- **Despatch (radio):** Make your way into the compound, an' take down Banes' workmen before they finish the job. They're all fanatical Bane followers, so expect severe resistance! *(at the start)*
- **Islander:** Officer, you gotta help us get out of here - those crazies are around the front! *(on talking to Islander)*
- **Islander:** Where the Hell's the Army when you need 'em? *(on talking to Islander)*
- **Despatch (radio):** Island residents have already been killed. Ensure that NO more come to harm! *(at the start)*
- **Despatch (radio):** Lord of the Jungle! *(after the AK-47 #139)*

</details>

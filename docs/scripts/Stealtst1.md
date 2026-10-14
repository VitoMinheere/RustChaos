# Seek, Sneak and Seize

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Stealtst1.ucm`, map `stealth1.iam`, district Medical Centre

**Played as:** Darci

**Briefing:**

> D'arci, I spent an hour last night tearing Bane's estate apart, and came up empty. I ripped out all the drives from his computer network and went to work on them last night. Found me an encrypted file, but damn it if I can't break the code - it's like nothing' I've ever seen. This must be the evidence that Bane is in league with higher forces than even I'd dreamed. What I got is one half of the encryption key. The other half is buried deep in a computer somewhere in the Gaemin Research Lab - a shady jet engine research centre bankrolled by an off-shore trust fund in Bane's name. We needed this file yesterday - get to it, soldier.

**Story:** Roper searched Bane's estate, pulled the drives and found an encrypted file, 'evidence that Bane is in league with higher forces', but has only half the key. The other half is in the Gaemin Research Center. Darci sneaks in (firing a gun raises the alarm), steals the confidential file, and meets Roper at the front gates in his van. Two guards argue over whether Bane really means 'Armageddon' and the missiles.

**Characters:**

- **Darci Stern**
- **Roper**: waits with the van
- Wildcat guards, one a true believer
- Men In Black

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 2 | Guard | Wildcat (red) | guard |  |
| 29 | Roper | Roper | driver |  |
| 83 | Doubting Wildcat | Wildcat (grey) | guard |  |
| 84 | Believing Wildcat | Wildcat (grey) | guard |  |

Others: 9 Wildcat (red) (guard); 4 Wildcat (grey) (guard); 3 Wildcat (red) (gang member); 2 Wildcat (rasta) (guard); 2 Man In Black (Man In Black); 1 Wildcat (grey, AK-47) (guard); 1 Wildcat (rasta, pistol) (guard); 1 Wildcat (rasta) (gang member).

</details>

**Objects:**

- Items: file, AK-47
- Vehicles: van (unlocked)

**Badges:** 10

**Beacons (places and people the player is sent to):** "Roper.", "Confidential File"

**How it ends:**

- **Won** after "Way to go, lets go.".
- **Lost** when the van #28 is wrecked.

<details><summary>Script</summary>

- **Darci:** I gotta get into that Research Center an' get that encryption code. Roper better be right about this - if I get caught....... *(at the start)*

*Conversation (on entering an area):*

> **Believing Wildcat:** You hear about the future, brother?  
> **Doubting Wildcat:** The future? You mean the missiles?  
> **Believing Wildcat:** WE are the future, brother. WE shall inherit the meek, and RULE the earth.  
> **Doubting Wildcat:** Hey man, maybe you been taking Bane a bit too seriously.. he's only bullshittin' when he talks about Armageddon an' all that, man!  
> **Believing Wildcat:** Have faith brother - for I have seen the TRUTH! ALL city-scum will BURN, man!  

- **Roper:** You got the disk. Come meet me at the front gates and lets get the hell out! *(on picking up an item)*
- **Guard:** INTRUDER! GET EM! *(when the player fires a gun)*
- **Despatch (radio):** Way to go, lets go. *(once both: on entering an area; and "You got the disk. Come meet me at the fr...")*
- **Despatch (radio):** Gate Open! *(after #49 (activate prim))*

</details>

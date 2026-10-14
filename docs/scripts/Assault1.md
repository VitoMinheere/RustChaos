# Physical Training

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\Assault1.ucm`, map `assault.iam`, district Assault Course

**Played as:** Darci

**Briefing:**

> Officer Dean will brief you on the requirements of this training exercise.

**Story:** Officer Dean puts Darci through her physical on the assault course: stick to the track, grab the zip cable, and hit the switch at the end before the two-minute clock runs out. Attacking Dean ends the test.

**Characters:**

- **Darci Stern**
- **Officer Dean**: the course instructor
- Sixteen cops lining the course

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 4 | Officer Dean | cop | civilian | stands still |

Others: 16 cop (cop).

</details>

**Objects:**

- Street furniture: traffic cone x9

**Beacons (places and people the player is sent to):** "FINISH"

**How it ends:**

- **Lost** after "OUT OF TIME!".
- **Lost** after "You CRAZY?".
- **Won** after "STOP THE CLOCK!".

<details><summary>Script</summary>

- **Despatch (radio):** STOP THE CLOCK! *(on pressing action here)*
- **Despatch (radio):** Jump up to grab that cable! *(on entering an area)*
- **Despatch (radio):** HIT THE SWITCH! *(on entering an area)*
- **Officer Dean:** Okay Stern, this is your physical! I want you to negotiate the assault course within the time limit! *(at the start)*

*Conversation (at the start):*

> **Officer Dean:** Okay Stern you ready?  
> **Darci:** Let me at it!  
> **Officer Dean:** GO FOR IT!  

- **Despatch (radio):** You CRAZY? *(when Officer Dean is down)*
- **Officer Dean:** ...for this test, you will be restricted to the course track, you will NOT be allowed to take shortcuts. *(at the start)*
- **Officer Dean:** .. At the end of the course, you MUST press the switch to stop the timer.. Good luck! *(at the start)*
- **Despatch (radio):** OUT OF TIME! *(on a 120s countdown (after the conversation "Okay Stern you ready?"))*

</details>

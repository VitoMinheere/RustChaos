# Combat Tutorial

*A released mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\FTutor1.ucm`, map `fight1.iam`, district Combat Centre

**Played as:** Darci

**Briefing:**

> Follow the instructions given, in order to learn hand to hand combat.

**Story:** Darci's first day at the Combat Centre. Text instructions walk her through punches and kicks, the grapple, sprinting, the sliding tackle, triple combos, knocking a target down and arresting it, and finally fighting four at once with back and side kicks. The targets are training dummies dressed as Wildcats.

**Characters:**

- **Darci Stern**: rookie in training
- Training dummies (Wildcat grey), who go down only to the move each lesson teaches

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 19 | Dummy 1 | Wildcat (grey) | training dummy | stands still |
| 22 | Dummy 2 | Wildcat (grey) | training dummy |  |
| 26 | Dummy 3 | Wildcat (grey) | training dummy |  |
| 28 | Dummy 4 | Wildcat (grey) | training dummy |  |
| 30 | Dummy 5 (arrest) | Wildcat (grey) | training dummy | invulnerable |
| 32 | Dummy 6 | Wildcat (grey) | training dummy | invulnerable |
| 33 | Dummy 7 | Wildcat (grey) | training dummy | invulnerable |
| 34 | Dummy 8 | Wildcat (grey) | training dummy | invulnerable |
| 35 | Dummy 9 | Wildcat (grey) | training dummy | invulnerable |

</details>

**Beacons (places and people the player is sent to):** "Target"

**How it ends:**

- **Won** after "Well done. Combat tutorial complete.".

<details><summary>Script</summary>

- **Instructions:** Welcome to the hand to hand tutorial, by the end of this tutorial you will have learnt how to eliminate or arrest multiple adversaries using a variety of moves. Firstly use the kick and punch buttons to try out the basic moves. *(at the start)*
- **Instructions:** Good work. Now get closer to the guy you're facing and press punch while moving forward. This will get him into a grapple. *(when a count is reached)*
- **Instructions:** Well done. Now use your GPS to approach the target. *(after punching and kicking)*
- **Instructions:** Good. Now either press punch or kick to perform a grapple move which will down this opponent. *(once both: when Darci is grabbed; and earlier events)*
- **Instructions:** Use your 'action' key to sprint while running. *(5s later (after Dummy 1))*
- **Instructions:** Well done, now approach the second target. *(when Dummy 1 is killed (not arrested))*
- **Instructions:** Perform a sliding tackle on the target by pressing kick while running. In a sprint, your slide will have a much longer range. *(once both: when the player gets near; and Dummy 1)*
- **Instructions:** Well done but we'll try that again on another target just to make sure you've got the timings right. *(when Dummy 2 is killed (not arrested))*
- **Instructions:** Perform a triple punch combination on this target by pressing the punch key 3 times. You will need to time the punches correctly. *(once both: when the player gets near; and Dummy 2)*
- **Instructions:** Good. Now approach the next target and use the kick button to perform a triple kick combo. *(when Dummy 3 is killed (not arrested))*
- **Instructions:** Well done, combinations of kicks and punches with the correct timings will enable you to perform more complex combos. Approach the next target. *(when Dummy 4 is killed (not arrested))*
- **Instructions:** When you reach the target, bring him down any way you wish. The action key on a downed enemy will perform an arrest. Arrest the target. *(4s later (after "Well done, combinations of kicks and pun..."))*
- **Instructions:** Good work, now approach the 4 targets indicated. *(when Dummy 5 (arrest) is arrested)*
- **Instructions:** Now you're in combat with these guys, I want you to use back and side kicks to take out the enemies to your rear and sides. *(once both: when the player gets near; and Dummy 9)*
- **Instructions:** If things get a little rough, try pressing left or right with jump to escape the fight. *(5s later (after "Now you're in combat with these guys, I ..."))*
- **Instructions:** Well done. Combat tutorial complete. *(once both: earlier events; and once both: earlier events; and earlier events)*

</details>

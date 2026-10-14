# Arms Breaker *(German; `wstores1D`)*

*An unreleased mission of Urban Chaos. Back to [the story overview](../STORY.md).*

`levels\wstores1D.ucm`, map `gpost3.iam`

**Played as:** Darci

**Story:** The same as `wstoresD`, translated into German.

<details><summary>Everyone the script makes</summary>

| Point | Who | Type | Role | Notes |
|---|---|---|---|---|
| 8 | Cabbie | civilian | civilian |  |
| 18 | Wildcat gossip | Wildcat (red) | stands about | stands still |
| 19 | Wildcat gossip | Wildcat (grey) | stands about | stands still |
| 21 | Alien hunter | workman | civilian |  |
| 22 | Working girl | woman | civilian |  |
| 65 | Working girl | large woman | civilian |  |
| 67 | Working girl | woman | civilian |  |
| 96 | Pick-up team | Wildcat (rasta) | guard |  |
| 97 | Pick-up team | Wildcat (red) | guard |  |
| 162 | Spinks | Wildcat (grey) | guard | stands still |
| 163 | Wildcat guard | Wildcat (red, shotgun) | guard | stands still |
| 174 | Tip-off citizen | civilian | civilian |  |
| 202 | 'Bloody' Mary Schwartz | large woman | civilian |  |
| 230 | Working girl | woman | civilian |  |
| 232 | Hot dog vendor | civilian | civilian |  |
| 244 |  | Wildcat (rasta) | guard | stands still |
| 245 |  | Wildcat (rasta) | guard | stands still |
| 246 | Assault victim | civilian | civilian | stands still |
| 260 | Cop | cop (pistol) | cop |  |
| 261 | Cop | cop (pistol) | cop |  |
| 262 | Cop | cop (pistol) | cop |  |
| 291 | Roper | Roper | civilian | stands still |
| 341 | Man in the alley | civilian | civilian | stands still |
| 344 |  | Wildcat (rasta) | guard |  |
| 381 |  | civilian | civilian |  |
| 383 | Flasher | workman | civilian |  |
| 384 | Working girl | woman | civilian |  |
| 395 |  | Wildcat (rasta) | guard |  |
| 396 |  | Wildcat (rasta) | guard |  |
| 413 |  | Wildcat (grey) | guard |  |
| 414 |  | Wildcat (red) | guard |  |
| 417 | Barmaid | woman | civilian |  |
| 423 |  | civilian | civilian |  |

Others: 6 Wildcat (red, shotgun) (guard); 5 Wildcat (grey) (guard); 4 Wildcat (rasta, pistol) (guard); 4 Wildcat (red) (guard); 3 Wildcat (grey, AK-47) (guard); 2 Wildcat (rasta, pistol) (bully); 1 Wildcat (grey) (gang member); 1 cop (pistol) (driver); 1 civilian (driver); 1 cop (shotgun) (cop driver); 1 Wildcat (rasta) (guard); 1 civilian (civilian); 1 Wildcat (red, shotgun) (bully); 1 Wildcat (red) (driver).

</details>

**Objects:**

- Items: barrels x35, health pack x8, explosives x6, AK-47 ammo x5, pistol ammo x4, shotgun ammo x3, pistol x2, AK-47
- Vehicles: taxi (locked) x6, police car (unlocked) x2, van (unlocked) x2, van (locked) x2, ambulance (unlocked), van (red key)
- Street furniture: bin x29

**Badges:** 13. **Side objectives:**

- Side objective (1 badge)
- Side objective (1 badge)

**Beacons (places and people the player is sent to):** "Pick up team", "Roper", "Wildcat stores"

**How it ends:**

- **Lost** when Roper is down.
- **Won** once both: once both: "Gute Arbeit, Stern. Aber werd mir nur  n..."; and "Sabotage an den Lieferungen!"; and "Pick up team eliminated.".

<details><summary>Script</summary>

- **Despatch (radio):** Holly Vale *(on entering an area)*
- **Street sign:** Banner Avenue *(on entering an area)*
- **Street sign:** Reagan Plaza *(on entering an area)*
- **Street sign:** Rio Canal-Bootshaus *(on entering an area)*
- **Bonus:** Du hast sie, Stern! Gut gemacht! *(when the whole group is down)*
- **Despatch (radio):** Gute Arbeit, Stern. Aber werd mir nur  nicht ZU bermtig. *(when a prop is damaged)*

*Conversation (when the player gets near):*

> **Darci:** Das reicht, Ekelpaket, steck deinen  kleinen Freund ein und verschwinde.  Das ist doch keine ffentliche  Toilette hier!  
> **Man in the alley:** Heeeey - ich stell nur 'ne Stange Wasser  in die Ecke - oder sind Krperfunktionen  etwa AUCH schon illegal, hh?  
> **Darci:** Zisch ab, Widerling, bevor ich dich  in einen Streckverband stecke!  

- **Despatch (radio):** Sabotage an den Lieferungen! *(when a prop is damaged)*

*Conversation (when the player gets near):*

> **Darci:** HEY! ASSHOLE! YOU WANNA HAND THERE?.  
> **Flasher:** Wha...? Uuuh...Hey I'm sorry officer, I - uhh..  
> **Darci:** Get the hell outta here, sicko.  
> **Flasher:** Yes'm, I'm gone!  

- **Darci:** Hmmm, ist das ein lausiges Versteck!  Sieht allerdings aus, als wre  keiner da.... *(on entering an area)*

*Conversation (on talking to Cabbie):*

> **Cabbie:** Taxi gefllig?  
> **Darci:** Zisch ab, Kumpel.  

- **Bonus:** Gute Arbeit, Stern - schtze, wir sollten  dich auf der Station nicht mehr Grnschnabel nennen. Hee-hee... *(when 'Bloody' Mary Schwartz is arrested)*

*Conversation (on entering an area):*

> **Wildcat gossip:** Ich hre, uns ist so eine Bullenmieze  auf der Spur, echt beinhart... weit du da was drber?  
> **Wildcat gossip:** Nhh, nicht viel. Ich wei, da diese  Reporterin, Gordansky, uns beim  Rumschnffeln zu sehr auf den Pelz kriecht.  
> **Wildcat gossip:** Keine Sorge wegen dieser Kuh Gordansky, fr die haben wir uns schon was ausgedacht!  Unsere Leute in Southside...  


*Conversation (on entering an area):*

> **Tip-off citizen:** Officer! Ich hab gerade einen Kerl mit  einer Waffe die Gasse hier langrennen  sehen!  
> **Darci:** Entfernen Sie sich aus dem Gebiet -  das sieht nach rger aus!  

- **Alien hunter:** Ich suche nach fremden Lebensformen. WIBBEL! *(on talking to Alien hunter)*
- **Despatch (radio):** Verdammt, Stern! Deine BLUTRNSTIGKEIT  hat uns gerade einen Fall gekostet! *(when 'Bloody' Mary Schwartz is killed (not arrested))*
- **Working girl:** Zisch ab! Ich mach's nicht mit Bullen! *(on talking to Working girl)*
- **Working girl:** Ich mach's nicht mit Frauen, tut mir leid, Se! *(on talking to Working girl)*
- **Hot dog vendor:** Hot Dog, Officer? Ich knnt's ja ein bichen BIEGEN,  bis es wie ein DOUGHNUT aussieht, damit Sie sich  nicht KOMISCH vorkommen.. hah, hah! *(on talking to Hot dog vendor)*
- **Despatch (radio):** Meine Gte, Stern - bist wohl doch noch  ein Grnschnabel... *(when Assault victim is down)*
- **Assault victim:** Gott sei Dank! *(on talking to Assault victim)*
- **Cop:** Du machst dich ja ganz gut, Grnschnabel, aber das heit  noch lange nicht, da du mir in die Quere kommen kannst. *(on talking to Cop)*
- **Cop:** Da wollte mich doch gerade glatt so ein  verdammtes freches Gang-Mitglied mit einem  Baseball-Schlger verprgeln! Wenn ich den finde,  wird er aber SEIN BLAUES WUNDER ERLEBEN! *(on talking to Cop)*
- **Cop:** He, das ist nicht dein Revier - wie kommt's,  da der Chief DICH hierhergeschickt hat? *(on talking to Cop)*

*Conversation (on talking to Roper):*

> **Roper:** Darci! Schn, da es dir  gut geht!  
> **Darci:** Ich kann auf mich aufpassen, Mystery Man. Und wieso schleichst du mir nach? Da  kriegt man ja eine Gnsehaut...  
> **Roper:** Ich schleiche dir nicht nach, Darci   - wir folgen nur derselben Spur.  
> **Darci:** Weg? Was meinst du mit Weg?  
> **Roper:** Die Wildcats, Stern - sie sind mehr, als  du denkst. Sie sind nicht, was sie  scheinen. Daher brauchst du diese Dinge,  die ich dir mitgebracht habe...  

- **Wildcat (grey) #2:** Hat dir keiner gesagt, da wir hier das  Sagen haben, Bulle? Das ist unser Revier!  Und ziemlich bald wird hier ALLES UNSER  Gebiet sein! Heehee hee! *(when Darci is seen by Wildcat (grey) #2)*
- **Working girl:** Spter, Schtzchen... erstmal mu ich  mir meine MIETE verdienen, klar? *(on talking to Working girl)*
- **Working girl:** He, ich steh nicht auf Ladies, Mdel! *(on talking to Working girl)*
- **Civilian #381:** Das hier ist wirklich das Allerletzte! *(on talking to Civilian #381)*

*Conversation (after the conversation "HEY! ASSHOLE! YOU WANNA HAND THERE?."):*

> **Working girl:** Mist! Dauernd habt ihr Bullen uns  berufsttige Mdels auf dem Kieker! Hast du mal Feuer?  

- **Barmaid:** Tut mir leid, Schtzchen, fr heute ist geschlossen.  Versuch's morgen nochmal. *(on talking to Barmaid)*
- **Civilian #423:** Verdammt! *(on talking to Civilian #423)*
- **Despatch (radio):** Darci - wir haben definitive Angaben,  wo der Sprengstoff der Wildcats versteckt  ist. Du mut da rein und das Ganze in die  Luft jagen, bevor sie es sich holen! *(at the start)*
- **Wildcat gossip:** Pa AUF - EIN BULLE! *(after the conversation "Ich hre, uns ist so eine Bullenmieze  au...")*

*Conversation (on entering an area):*

> **Spinks:** Vergisses, Mann. Mist, wie sollen wir  uns denn verteidigen, wenn die Bullen  pltzlich zuschlagen?  
> **Wildcat guard:** Nur keine Panik, Spinks - unser Mann wei  schon, was er tut.  
> **Spinks:** Ach, und wer ist denn wohl "Unser Mann"?  
> **Wildcat guard:** Das wei ich nicht, und ich BRAUCHE das  auch nicht zu wissen - genau wie du. Also  hr schon mit dem Geseier auf!  
> **Spinks:** Verdammter Mist ... unsere Zeit sollte  besser BALD kommen, Mann, ich werd echt nervs.  

- **Despatch (radio):** Stern, die Nutte an der Bar ist "Bloody" Mary Schwartz! Wir haben in einem offenen  Fall noch ein paar Fragen an sie -  schlepp sie mal ab zur Station. *(when the player gets near)*
- **Despatch (radio):** Stern - ein berfall - tu was! *(once either: on entering an area; or on entering an area)*
- **Bonus:** Gefahr ausgeschaltet! *(once both: when Wildcat (rasta) #245 is down; and when Wildcat (rasta) #244 is down)*
- **Despatch (radio):** Stern? Roper hier. Triff mich an der Ecke Geebag Street. Ich hab ein paar Sachen, die  du brauchen knntest... *(at the start)*
- **Roper:** Darci - sieht aus, als wren die  Wildcats mit einem Wagen  gekommen - beeil dich und beseitige die  Hilfskrfte! *(3s later (once either: "Sabotage an den Lieferungen!"; or "Gute Arbeit, Stern. Aber werd mir nur  n..."))*
- **Wildcat (rasta) #344:** He, Weibsstck... wir haben euch Bullen  doch gesagt, ihr HABT VERDAMMT NOCHMAL  IN UNSEREM REVIER NICHTS ZU SUCHEN!  Jetzt kannst du den Ehrengast bei unseren  Feiern spielen! *(after Wildcat (rasta) #344)*
- **Despatch (radio):** Pick up team eliminated. *(once both: once both: when Wildcat (rasta) #396 is down; and when Wildcat (rasta) #395 is down; and once both: when Pick-up team is down; and when Pick-up team is down)*

</details>

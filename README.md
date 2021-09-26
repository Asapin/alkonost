# Alkonost

Simple console spam detector for YouTube chats.

Monitors a set of YouTube channels, and starts collecting messages as soon as a new chat room opens. All collected messages are then sent to an embedded spam detector and also saved to a database for a further analysis when searching for a ways to improve spam detection.

Consists of several modules:
* `core` - common types and objects used by other modules
* `StreamFinder` - monitors YouTube channels for live and upcoming streams and premiers
* `ChatManager` - collects messages from every open chat room
* `Detector` - analyses messages and tries to detect potential spammers
* `CLI` - temporary CLI front end
* `DB` - saves all messages and desicions, made by `Detector` to a database (module is not implemented yet)
* `WEB` - web front end, that allows to add and remove channels from monitoring and to change various aspects of spam detection (module is not implemented yet)

All modules, except `core` are implemented as independend actors, which should allow for an easy horizontal scallability in the future, if such a need ever arises.
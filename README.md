# Alkonost

Simple console spam detector for YouTube chats.

Monitors a set of YouTube channels, and starts collecting messages as soon as a new chat room opens. All collected messages are then sent to an embedded spam detector and also saved to a database for a further analysis when searching for a ways to improve spam detection.

Consists of several modules:
* [core](core/) - common types and objects used by other modules
* [StreamFinder](stream_finder/) - monitors YouTube channels for live and upcoming streams and premiers
* [ChatManager](chat_manager/) - collects messages from every open chat room
* [Detector](detector/) - analyses messages and tries to detect potential spammers
* [CLI](cli/) - temporary CLI front end
* `DB` - saves all messages and desicions, made by `Detector` to a database (module is not implemented yet)
* `WEB` - web front end, that allows to add and remove channels from monitoring and to change various aspects of spam detection (module is not implemented yet)

All modules, except `core` are implemented as independend actors, which should allow for an easy horizontal scallability in the future, if such a need ever arises.

## YouTube API

This app doesn't use YouTube API, and instead opted to emulate the behaviour of a browser with the chat being open. The reason for this decision is that YouTube by default provides only 10 thousand credits a day to spend on requests. 1 request to load new messages from the chat costs [5 credits][1], and, depending on how active the chat, should be performed every 5-10 seconds. If we assume that on average the app would perform 5 requests per minute, we can estimate that we will spend around 1500 credits per hour. 

[1]: https://stackoverflow.com/a/67745370

And some channels either stream for 24/7, for example [Lofi Girl](https://www.youtube.com/channel/UCSJ4gkVC6NrvII8umztf0Ow), or have streams planned far into the future, that effectively act as a chat rooms for viewers without the need to create Discord server. That's ~36000 credits per day for each such stream/chat room.

Moreover, to get the list of live broadcasts, we would have to use [Search API](https://developers.google.com/youtube/v3/docs/search/list), which costs 100 credist for each request, meaning that we can only make 1 request every ~15 minutes. And then we would have only 400 credits left to actually collect chat messages. And that's only for 1 channel.
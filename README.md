# Alkonost

Simple console spam detector for YouTube chats.

Monitors a set of YouTube channels, and starts collecting messages as soon as a new chat room opens. All collected messages are then sent to an embedded spam detector and also saved to a database for a further analysis when searching for a ways to improve spam detection.

Consists of several modules:
* [shared](shared/) - common types and objects used by other modules
* [StreamFinder](stream_finder/) - monitors YouTube channels for airing and upcoming streams and premiers
* [ChatPoller](chat_poller/) - loads messages from the YouTube chat
* [ChatManager](chat_manager/) - collects messages from every open chat room
* [Detector](detector/) - analyses messages and tries to detect potential spammers
* `DB` - saves all messages and desicions, made by `Detector` to a database (module is not implemented yet)
* [Alkonost](alkonost/) - main library, responsible for creating all other modules and re-exporting only functionality, that should be used by UI implementation
* [UI](ui/) - a collection of UI implementations for `Alkonost`

All modules, except `shared` are implemented as independend actors, which should allow for an easy horizontal scallability in the future, if such a need ever arises.

## YouTube API

This app doesn't use YouTube API, and instead tries to emulate the behaviour of a browser. The reason for this decision is that YouTube by default provides only 10 thousand credits a day to spend on requests. 1 request to load new messages from the chat costs [5 credits][1], and, depending on how active the chat, should be performed every 5-10 seconds. If we assume that on average the app would perform 5 requests per minute, we can estimate that we will spend around 1500 credits per hour. 

[1]: https://stackoverflow.com/a/67745370

And some channels either stream for 24/7, for example [Lofi Girl](https://www.youtube.com/channel/UCSJ4gkVC6NrvII8umztf0Ow), or have streams planned far into the future, that effectively act as a chat rooms for viewers without the need to create Discord server. That's ~36000 credits per day for each such stream/chat room.

Moreover, to get the list of live broadcasts, we would have to use [Search API](https://developers.google.com/youtube/v3/docs/search/list), which costs 100 credist for each request, meaning that we can only make 1 request every ~15 minutes. And then we would have only 400 credits left to actually collect chat messages. And that's only for 1 channel.

## Setup

Before using the app, you need to provide several settings: spam detection parameters (`DetectorParams` struct), user agent to use when making HTTP-requests (`RequestSettings` struct) and a frequency of how often the app should check for new streams.

For now all those setting are hardcoded inside each UI implementation, but eventually they all should be loaded from a database, and should be accessible for modification at runtime through UI.

You also need to provide a list of channels to track. The app is using `channel id` when adding a new channel, but some YouTube channels use custom user name instead of channel id (e.g. https://www.youtube.com/user/PewDiePie). In that case you need to open any video from the channel, and then click on the channel's name under the video. This would open the same channel page, but this time instead of custom user name, you'll see channel id in the browser's address bar (e.g. https://www.youtube.com/channel/UC-lHJZR3Gqxm24_Vd_AJ5Yw for PewDiePie).

To see what exactly you need to do, when implementing new UI, please check how simple [CLI UI](ui/src/bin/cli.rs) is implemented.
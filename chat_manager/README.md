# Chat manager
This module is responsible for keeping track of all open chat rooms, loading messages from them, and redirecting those messages to both `SpamDetector` and `DB` (the latter is not implemented yet).

Can be easily transformed to a standalone app, if the amount of request from a single computer becomes too high to the point when it triggers YouTube's anti-ddos protection. All that is needed to be done in that case is to replace incoming and outgoing channels with RabbitMQ channels or something similar.

Can also be easily scaled horizontally if placed behind a balancing router. Router should ensure, that the same stream id inside the message `FoundStreamIds(HashSet<String>)` would be always sent to the same instance.

## How it works

During the initialization process, the module creates two proxies in two separate Tokio tasks. 
* The first proxy is responsible for receiving `ChatManagerMessages` messages from the `StreamFinder` module, converting them to `ManagerMessages`, and resending them to the inner `manager_tx` channel.
* The second proxy is responsible for receiving `PollingResultMessages` messages from all existing `ChatPoller`'s, also converting them to `ManagerMessages`, and resending them to the inner `manager_tx` channel.

The module itself reads all those messages from the single `manager_rx` channel, initializes `ChatPoller`'s for new streams, updates request parameters in all existing `ChatPoller`'s, and resends all chat messages, received from existing `ChatPoller`'s, to the `SpamDetector` (and to the `DB` in the future).

Upon receiving `Close` message or on encountering an unrecoverable error, `ChatManager` will try to gracefully close all `ChatPoller`'s, resend all remaining `NewMessages` and `StreamEnded` messages to the `SpamDetector` (and to the `DB` in the future), shutdown both proxies, and send `Close` message to the `SpamDetector` (and to the `DB` in the future).

### Possible incoming messages from the StreamFinder

* `FoundStreamIds(HashSet<String>)` - list of all upcoming and live streams and premiers
* `UpdateUserAgent(String)` - update user agent, that's used when making GET and POST request to YouTube
* `UpdateBrowserVersion(String)` - update browser version, that's gets sent to YouTube
* `UpdateBrowserNameAndVersion { name: String, version: String }` - update both browser name and version, that gets sent to YouTube
* `Close` - interrupt the processing loop, effectivly terminating the execution of the module

Messages `UpdateUserAgent`, `UpdateBrowserVersion` and `UpdateBrowserNameAndVersion` are also retranslated to all existing `ChatPoller`'s.

### Possible incoming messages from the ChatPoller

* `NewBatch { video_id: String, actions: Vec<Action> }` - list of new messages in the chat
* `StreamEnded { video_id: String }` - indicates that the chat has been closed

Both of those messages are retranslated to the `SpamDetector` (and to the `DB` in the future).

## Chat poller

Responsible for actually loading messages from the YouTube chat by performing a POST-request to the `https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key=<chat_key>` url. The response is a JSON string, that contains: 

* all messages since the last request
* `continuation` parameter, that must be sent during the next POST-request
* `timeout_ms` parameter, that indicates how long the `ChatPoller` should wait, before making another POST-requst

During the initialization process, `ChatPoller` first makes a GET-request to the `https://www.youtube.com/live_chat?is_popout=1&v=<video_id>` url, which return an HTML-page with embedded JSON string inside. This JSON string actually contains two `continuation` parameters: using the first one would result in loading messages, marked by YouTube as `Top chat`, while the second one will load messages, marked as `Live chat`.

After the initialization, all other request will return either only one `continuation` parameter, or no `continuation` at all, which indicates, that the chat has been closed.

### Existing bugs/errors

Attempting to load new messages from the chat would result sometimes in a `Broken pipe` error. It's a somewhat rare error, occuring only 3-4 times during the 10-12 hours of collecting messages, and I'm not sure if it's a bug in the `reqwest` library, if it's a Windows-specific bug, or if it's a problem with YouTube.

Regardless of whose fault it is, the `ChatPoller` will perform 3 attempts to load new messages, waiting 100ms between each request. If it fails 3 times in a row, than the `ChatPoller` would be considered broken, and would be closed. Every successful request would resend the amount of attempts left back to 3.

Also, if the `ChatPoller` encounters any error during the deserialization of a JSON-string, it will log the error, and save incoming JSON-string to a file for further analysis.

## Possible future improvements

* Replace all `println!()` calls with the use of a proper logging framework
* Upon receiving `Close` message, return the underlying data like `chat_params`, `next_poll_time` and so on of every existing `ChatPoller`, for a potential migration to another instance
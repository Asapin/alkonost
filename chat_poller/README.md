# Chat Poller
This module is responsible for loading messages from a specific YouTube chat and sending them to `result_tx`.

Can be easily transformed to a standalone app, if the amount of requests from a single computer becomes too high to the point when it triggers YouTube's anti-ddos protection. All that is needed to be done in that case is to replace incoming and outgoing channels with RabbitMQ channels or something similar.

## How it works

During the initialization process, the module loads HTML-page from the `https://www.youtube.com/live_chat?is_popout=1&v=<video_id>`, which contains several parameters that are used for loading actual messages. It also contains two different parameters, named `continuation`. Using the first one would result in loading messages, marked by YouTube as `Top chat`, while the second one will load messages, marked as `Live chat`.

If initialization was successful it sends `OutMessages::ChatInit` and starts a new Tokio task, that reads messages from an MPSC `rx` channel until `next_poll_time` in an endless loop. When the deadline is reached, this task pulls new messages from the YouTube chat and sends them to `result_tx`.

To load new messages, the module makes a POST-request to the `https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key=<chat_key>` url. The response is a JSON string, that contains: 

* all messages since the last request
* `continuation` parameter, that must be sent during the next POST-request
* `timeout_ms` parameter, that indicates how long the `ChatPoller` should wait, before making another POST-requst

If the response doesn't contain the `continuation` parameter, then it means either that the stream has ended or the chat was disabled.

### Possible incoming MPSC messages

* `UpdateUserAgent(String)` - update user agent, that's used when making GET and POST request to YouTube
* `UpdateBrowserVersion(String)` - update browser version, that's gets sent to YouTube (not used in this module)
* `UpdateBrowserNameAndVersion { name: String, version: String }` - update both browser name and version, that gets sent to YouTube (not used in this module)
* `Close` - interrupt the processing loop, effectivly terminating the execution of the module

## Existing bugs/errors

Attempting to load new messages from the chat would result sometimes in a `Broken pipe` error. It's a somewhat rare error, occuring only 3-4 times during the 10-12 hours of collecting messages, and I'm not sure if it's a bug in the `reqwest` library, if it's a Windows-specific bug, or if it's a problem with YouTube.

Regardless of whose fault it is, the `ChatPoller` will perform 3 attempts to load new messages, waiting 100ms between each request. If it fails 3 times in a row, than the `ChatPoller` would be considered broken, and would be closed. Every successful request would resend the amount of attempts left back to 3.

Also, if the `ChatPoller` encounters any error during the deserialization of a JSON-string, it will log the error, and save incoming JSON-string to a file for further analysis.

## Possible future improvements

* Replace all `println!()` calls with the use of a proper logging framework
* Upon receiving `Close` message, return the underlying data like `chat_params`, `next_poll_time` and so on of every existing `ChatPoller`, for a potential migration to another instance
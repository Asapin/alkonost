# Chat manager
This module is responsible for keeping track of all open chat rooms and creating new chat pollers.

Can be easily transformed to a standalone app, if the amount of request from a single computer becomes too high to the point when it triggers YouTube's anti-ddos protection. All that is needed to be done in that case is to replace incoming and outgoing channels with RabbitMQ channels or something similar.

Doesn't need any horizontal scaling, as all it does is just creates and controls chat pollers.

## How it works

During the initialization process, the module creates a new Tokio task, which reads incoming messages from an MPSC channel named `rx` until the `check_children_period` has passed in an endless loop. When the deadline is reached, this task checks if any of the active chat pollers has notified about its closure, and removes such pollers.

### Possible incoming messages

* `FoundStreamIds(HashSet<String>)` - list of all upcoming and live streams and premiers
* `UpdateUserAgent(String)` - update user agent, that's used when making GET and POST request to YouTube
* `UpdateBrowserVersion(String)` - update browser version, that's gets sent to YouTube
* `UpdateBrowserNameAndVersion { name: String, version: String }` - update both browser name and version, that gets sent to YouTube
* `Close` - interrupt the processing loop, effectivly terminating the execution of the module

Messages `UpdateUserAgent`, `UpdateBrowserVersion` and `UpdateBrowserNameAndVersion` are also retranslated to all existing `ChatPoller`'s.

### Existing bugs/errors

None that I know of.

## Possible future improvements

* Replace all `println!()` calls with the use of a proper logging framework
* Upon receiving `Close` message, return the underlying data like `chat_params`, `next_poll_time` and so on of every existing `ChatPoller`, for a potential migration to another instance
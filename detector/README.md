# Spam detector
This module is responsible for processing extracted messages and detecting potential spamers.

Can be easily transformed to a standalone app, if the amount of messages to process becomes too high or if the processing becomes too complex and messages start to back up. All that is needed to be done in that case is to replace incoming and outgoing channels with RabbitMQ channels or something similar.

Can also be easily scaled horizontally if placed behind a balancing router. Router should ensure, that messages with the same video id would be always sent to the same instance.

## How it works

Each live and upcoming stream and premier has its own separate instance of a spam detector. Upon receiving a new batch of messages, detector manager loads an instance, responsible for that chat, or creates a new one if it doesn't exist, and delegates the actual processing to that instance. The result of the processing is a list of decisions made by that instance.

All decisions are then sent to the frontend to be presented to the users. When the stream ends, the manager removes respective instance, and resends `StreamEnded` message to the front end.

### Possible incoming messages from the ChatManager

* `NewBatch { video_id: String, actions: Vec<Action> }` - new messages from the `video_id` chat
* `StreamEnded { video_id: String }` - indicates that the chat has been closed
* `Close` - interrupt the processing loop, effectivly terminating the execution of the module

### Spam detection

Because the probability of a *moderator*, a *member* or a *verified* user being an actual spammer is basically non-existent, messages from these users are **not** processed. Additionally, if a user has sent a superchat during the stream, they are marked as a channel supporter, and spam detector also stops processing their messages. Detector also skips all users, who already marked as potential spammers. All these optimizations greatly reduce the amount of needed memory and CPU.

Suspicion triggers:

* Too many deleted messages - usually, users don't delete their messages, but some spammers delete their messages after a few seconds as an attempt to protect their channel from an early termination. The streamer/moderators can still easily ban spammers, who deleted their messages, but regular viewers can't report deleted messages to YouTube moderator team.
* Average message length - greatly depends on the language used, and the streamer themself, but usually messages from regular users are quite short, while spammers can send very big messages in an attempt to make the chat unusable for other viewers.
* Average delay between messages - if spammers use a macro to spam, they can send messages in a very quick succession, and even break YouTube's slow mode, as it is (at least used to be) implemented client side
* Too many similar messages

## Existing bugs/errors

Not that I'm aware of.

## Possible future improvements

* Replace all `println!()` calls with the use of a proper logging framework
* Upon receiving `Close` message, return the underlying data like `streams`, `params` and so on, for a potential migration to another instance
* Separate `DetectorParams` for every stream and channel
* Analyze messages and user names for offensive and blacklisted words
* Detect when streamer enables/disables slow mode, and dynamically adjust the `avg_delay` threshold
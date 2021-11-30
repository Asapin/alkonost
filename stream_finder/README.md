# Stream Finder
This module is responsible for searching airing and upcoming live streams and premiers on specified YouTube channels. 

Can be easily transformed to a standalone app, if the amount of requests from a single computer becomes too high to the point when it triggers YouTube's anti-ddos protection. All that is needed to be done in that case is to replace incoming and outgoing channels with RabbitMQ channels or something similar.

Can also be easily scaled horizontally if placed behind a balancing router. Router have to ensure, that messages `AddChannel(String)` and `RemoveChannel(String)` for the same channel will always be sent to the same instance, for example by having a hash map `channel_id -> instance_id`.

## How it works

During the initialization process, the module creates a new Tokio task, which reads incoming messages from an MPSC channel named `rx` until the `next_poll_time` in an endless loop. When the deadline is reached, this task loads in parallel the content of every channel it tracks, using `FuturesUnordered`, and extracts live and upcoming streams and premiers and logs all encountered errors while doing so.

IDs of all found streams and premiers are then sent to the `result_tx` for further processing, and the `next_poll_time` is updated to `Instant::now() + self.poll_interval`.

### Possible incoming MPSC messages

* `AddChannel(String)` - add new channel for tracking airing and upcoming streams and premiers
* `RemoveChannel(String)` - remove the channel from tracking
* `UpdatePollInterval(u64)` - update polling interval, in miliseconds
* `UpdateUserAgent(String)` - update user agent, that's used when making GET and POST request to YouTube
* `UpdateBrowserVersion(String)` - update browser version, that's gets sent to YouTube (not used in this module)
* `UpdateBrowserNameAndVersion { name: String, version: String }` - update both browser name and version, that gets sent to YouTube (not used in this module)
* `Close` - interrupt the processing loop, effectivly terminating the execution of the module

### Extracting upcoming and live streams and premiers

To extract the list of upcoming and live streams and premiers from a channel, the module downloads its HTML content by making a GET-request to the `https://www.youtube.com/channel/<channel_id>/videos?view=57` (the option `All videos` in the dropdown menu on the `VIDEOS` tab), and extracts JSON data using regex.
The resulting JSON has quite a complex structure, but all that we need from it are video entries, that **don't** have `publishedTimeText` field, which indicates when the video was published or streamed in case of streams and premiers.

If the module encounters a deserialization error, the content of the loaded HTML page would be dumped into `<channel_id>.channel` file for a future analisys. 

## Existing bugs/errors

Sometimes loading a channel page would result in an HTML, that has a slightly different content with the data about videos nowhere to be found. It's generally not a problem, since the channel would be probed again during the next cycle, but nonetheless the reason why it happens remains a mystery. All such occurences are logged, and the response is dumped into a file for a future analysis.

## Possible future improvements

* Upon receiving `Close` message, return the underlying data like `poll_interval`, `channels` and so on, for a potential migration to another instance
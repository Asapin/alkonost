# Alkonost

Main library, responsible for creating and setting up `StreamFinder`, `ChatManager` and `Detector`. Exposes only channels for incoming and outgoing messages and a custom handler to join on when trying to gracefully close an application. Should be the main dependency for anyone who tries to implement a UI.
# KeyBot

## Description

KeyBot is an automated system designed to manage and distribute keys (e.g., software license keys, access tokens) efficiently through a Discord bot. It ensures that keys are only claimed once and tracks the distribution to users. The bot is particularly useful for giveaways, ensuring a fair and organized distribution of keys among participants. It operates by monitoring for key claim requests and processing these requests according to predefined rules and the availability of unclaimed keys.

## Features

- **Automated Key Distribution**: Automates the process of distributing keys to users, ensuring each key is claimed only once.
- **Active Round Management**: Supports the concept of "rounds" for giveaways, allowing for organized distribution events.
- **User Tracking**: Tracks which users have claimed keys, preventing multiple claims by the same user in a given round.
- **Configurable**: Can be customized via a configuration file to suit different needs and scenarios.

## Configuration

KeyBot is configured through a simple configuration file (`config.json5`), which allows you to specify various operational parameters such as database connection details, the maximum number of keys a user can claim, and other bot settings.

### Config File Structure

The configuration file contains key-value pairs. Here is an example structure for `config.toml`:

```json5
{
  // Default key giveaway duration in seconds
  // This can be overridden by the giveaway_duration argument
  giveaway_duration: 3600,
  // The age of the account required to claim a key
  // given in days
  age_bound: 5
}
```

## Upcoming

- No changes so far

# v0.2.4

- Add auto pp recalculate (for peace)
- If pp calculation fails (such as restarting pp-server), just save task to redis in the format of "`calc:{table(mode)}:{score_id}:{player_id}`":"`md5=xxx&mods=xx&mode=xx&n300=xx`". pp-server will auto recalculate these tasks, and notify peace to update the stats of these players.

# v0.2.3

- Add no_miss option, (additional) calculate score pp if no miss.

# v0.2.2

- More info.
- Now, we can get Raw pp result: aim, spd, acc, str.
- And, we can get stars, mods, mode of pp results.
- Support for calculate acc list pp. (As Tillerino)
- We can choose simple info or raw info.

# v0.2.0

- Big updates.
- Now, we can get beatmap from osu!api, download .osu files.
- Add feature "peace", we can use postgresql database to cache beatmap.
- Support for beatmap id.
- Support for beatmapset id + file name.
- Support for more params.

## v0.1.0

- Initial version.
- Basic ability, Calculate the pp according to local beatmap.
- Simple api.

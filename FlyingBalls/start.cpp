
#include "game.h"
#include "start.h"

extern "C" int start_game(int width, int height) {

    Game *game = new Game(width, height);
    game->init();

    while (game->running()) {
        game->handleEvents();
        game->update();
        game->render();
    }

    return 0;
}
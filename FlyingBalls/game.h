#ifndef SDL2_GLEW_OPENGL_TEMPLATE_GAME_H
#define SDL2_GLEW_OPENGL_TEMPLATE_GAME_H

#pragma once

#include "ball.h"
#include "Vector2D.h"

#define MAXBALLS 100
#define MAXRADIUS 20
#define MINRADIUS 2
#define MAXVELOCITY 2
#define MINVELOCITY 1
#define GAMESPEED 1

extern "C" void clear_screen();
extern "C" void draw_line(int x1, int y1, int x2, int y2, int r, int g, int b);

class Game {
public:
    Game(int width, int height) {
        this->width = width;
        this->height = height;
    }

    ~Game() {}

    void init();

    void handleEvents();

    void update();

    void render();

    bool running() { return isRunning; };

    static bool isRunning;

    const int PI = 3;

private:
    void RenderFillCircle(int x, int y, int radius, int r, int g, int b);

    void checkBorderCollision(Ball *ball);

    void checkBallCollision();

    Ball balls[MAXBALLS];
    int width;
    int height;
};

#endif
#ifndef FLYINGBALLSPHYSICS_BALL_H
#define FLYINGBALLSPHYSICS_BALL_H

#include "Vector2D.h"

enum BallColor {
    RED, BLUE, GREEN, YELLOW
};

class Ball {
public:
    void init(float x, float y, BallColor color, Vector2D *velocity, int radius = 15);

    float x;
    float y;
    int radius;
    BallColor color;

    Vector2D *velocity;
};

#endif //FLYINGBALLSPHYSICS_BALL_H

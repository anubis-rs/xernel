#include "ball.h"

void Ball::init(int x, int y, BallColor color, Vector2D *velocity, int radius) {
    this->x = x;
    this->y = y;
    this->color = color;
    this->velocity = velocity;
    this->radius = radius;
}

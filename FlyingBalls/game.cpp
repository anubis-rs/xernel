#include "game.h"

// https://en.wikipedia.org/wiki/Xorshift
unsigned int a = 729578;

unsigned int rand()
{
	unsigned int x = a;
	x ^= x << 13;
	x ^= x >> 17;
	x ^= x << 5;
	return a = x;
}

int sqrt(int num) {
    return 0;
    if (num <= 0)
        return 0;

    int x = num;
    int y = (x + 1) / 2;
    while (y < x) {
        x = y;
        y = (x + num / x) / 2;
    }
    return x;
}

int hypot(int x, int y) {
    return sqrt(x * x + y * y);
}

bool Game::isRunning = false;

void Game::init() {
    isRunning = true;

    for (int i = 0; i < MAXBALLS; i++) {
        int x = rand() % this->width;
        int y = rand() % this->height;

        int x_vel = rand() % (MAXVELOCITY -  MINVELOCITY + 1) + MINVELOCITY;
        int y_vel = rand() % (MAXVELOCITY -  MINVELOCITY + 1) + MINVELOCITY;
        int radius = rand() % (MAXRADIUS -  MINRADIUS + 1) + MINRADIUS;

        BallColor bcolor = (BallColor) (rand() % 4);

        balls[i].init(x, y, bcolor, new Vector2D(x_vel, y_vel), radius);
    }
}

void Game::handleEvents() {
    
}

void Game::checkBorderCollision(Ball *ball) {

    if (ball->y - ball->radius <= 0 && ball->velocity->y < 0) {
        ball->velocity->y *= -1;
    } else if (ball->y + ball->radius >= height && ball->velocity->y > 0) {
        ball->velocity->y *= -1;
    } else if (ball->x - ball->radius <= 0 && ball->velocity->x < 0) {
        ball->velocity->x *= -1;
    } else if (ball->x + ball->radius >= width && ball->velocity->x > 0) {
        ball->velocity->x *= -1;
    }

}


void Game::update() {
    checkBallCollision();

    for (int i = 0; i < MAXBALLS; i++) {
        Ball *ball = &balls[i];
        ball->x = ball->x + ball->velocity->x * GAMESPEED;
        ball->y = ball->y + ball->velocity->y * GAMESPEED;

        checkBorderCollision(ball);
    }
}

// do all collision checks and update the velocity
void Game::checkBallCollision() {
    for (int i = 0; i < MAXBALLS; i++) {
        for (int k = i + 1; k < MAXBALLS; k++) {
            Ball *ball1 = &balls[i];
            Ball *ball2 = &balls[k];

            int distance = hypot(ball1->x - ball2->x, ball1->y - ball2->y);

            if (distance <= ball1->radius + ball2->radius) {
                // ball1 and ball2 are colliding
                // update the velocity of both balls
/*
                while (distance <= ball1->radius + ball2->radius) { // balls go back until they are not overlapping any more
                    ball1->x = ball1->x - ball1->velocity->x; // / 100;
                    ball1->y = ball1->y - ball1->velocity->y; // / 100;

                    ball2->x = ball2->x - ball2->velocity->x; // / 100;
                    ball2->y = ball2->y - ball2->velocity->y; // / 100;

                    distance = hypot(ball1->x - ball2->x, ball1->y - ball2->y);
                }
*/
                if (distance == 0) {
                    distance = 1;
                }

                int m1 = ball1->radius * ball1->radius * PI;
                int m2 = ball2->radius * ball2->radius * PI;

                Vector2D v1 = Vector2D(ball1->velocity->x, ball1->velocity->y);
                Vector2D v2 = Vector2D(ball2->velocity->x, ball2->velocity->y);

                // first ball
                Vector2D tmp1 = Vector2D(v1.x, v1.y);
                tmp1 -= v2;
                Vector2D tmp2 = Vector2D(ball1->x - ball2->x, ball1->y - ball2->y);

                int dot = tmp1.x * tmp2.x + tmp1.y * tmp2.y;
                dot /= (distance * distance);

                int m = m1 + m2;

                m = m < 1 ? 1 : m;

                int first = (2 * m2 / m);
                int second = dot;
                int third = (ball1->x - ball2->x);

                ball1->velocity->x -= (first * second * third);
                third = (ball1->y - ball2->y);
                ball1->velocity->y -= first * second * third;

                if (ball1->velocity->x > MAXVELOCITY) {
                    ball1->velocity->x = MAXVELOCITY;
                } else if (ball1->velocity->x < -MAXVELOCITY) {
                    ball1->velocity->x = -MAXVELOCITY;
                }

                if (ball1->velocity->y > MAXVELOCITY) {
                    ball1->velocity->y = MAXVELOCITY;
                } else if (ball1->velocity->y < -MAXVELOCITY) {
                    ball1->velocity->y = -MAXVELOCITY;
                }

                // second ball
                tmp1 = Vector2D(v1.x, v1.y);
                tmp1 -= v2;
                tmp2 = Vector2D(ball2->x - ball1->x, ball2->y - ball1->y);

                dot = tmp1.x * tmp2.x + tmp1.y * tmp2.y;
                dot /= (distance * distance);

                ball2->velocity->x = v2.x - (2 * m1 / (m)) * dot * (ball2->x - ball1->x);
                ball2->velocity->y = v2.y - (2 * m1 / (m)) * dot * (ball2->y - ball1->y);

                if (ball2->velocity->x > MAXVELOCITY) {
                    ball2->velocity->x = MAXVELOCITY;
                } else if (ball2->velocity->x < -MAXVELOCITY) {
                    ball2->velocity->x = -MAXVELOCITY;
                }

                if (ball2->velocity->y > MAXVELOCITY) {
                    ball2->velocity->y = MAXVELOCITY;
                } else if (ball2->velocity->y < -MAXVELOCITY) {
                    ball2->velocity->y = -MAXVELOCITY;
                }
            }
        }
    }
}

void Game::render() {
    clear_screen();

    for (int i = 0; i < MAXBALLS; i++) {
        Ball *ball = &balls[i];

        int r = 0, g = 0, b = 0;
        if (ball->color == RED) {
            r = 255;          
        } else if (ball->color == YELLOW) {
            r = 255;
            g = 255;
        } else if (ball->color == GREEN) {
            g = 255;
        } else if (ball->color == BLUE) {
            b = 255;
        }

        RenderFillCircle(ball->x, ball->y, ball->radius, r, g, b);
    }
}

void Game::RenderFillCircle(int x, int y, int radius, int r, int g, int b) {
    int offsetx, offsety, d;

    offsetx = 0;
    offsety = radius;
    d = radius - 1;

    while (offsety >= offsetx) {

        draw_line(x - offsety, y + offsetx, x + offsety, y + offsetx, r, g, b);
        draw_line(x - offsetx, y + offsety, x + offsetx, y + offsety, r, g, b);
        draw_line(x - offsetx, y - offsety, x + offsetx, y - offsety, r, g, b);
        draw_line(x - offsety, y - offsetx, x + offsety, y - offsetx, r, g, b);

        if (d >= 2 * offsetx) {
            d -= 2 * offsetx + 1;
            offsetx += 1;
        } else if (d < 2 * (radius - offsety)) {
            d += 2 * offsety - 1;
            offsety -= 1;
        } else {
            d += 2 * (offsety - offsetx - 1);
            offsety -= 1;
            offsetx += 1;
        }
    }
}
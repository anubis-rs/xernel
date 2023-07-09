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

float sqrt(float x) {
    float xhalf = 0.5f * x;
    int i = *(int *) &x; // get bits for floating value
    i = 0x5f375a86 - (i >> 1); // gives initial guess y0
    x = *(float *) &i; // convert bits back to float
    x = x * (1.5f - xhalf * x * x); // Newton step, repeating increases accuracy
    return 1 / x;
}

float hypot(float x, float y) {
    return sqrt(x * x + y * y);
}

bool Game::isRunning = false;

void Game::init() {
    isRunning = true;

    for (int i = 0; i < MAXBALLS; i++) {
        int x = rand() % width;
        int y = rand() % height;

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
        ball->x = ball->x + ball->velocity->x * (float) GAMESPEED;
        ball->y = ball->y + ball->velocity->y * (float) GAMESPEED;

        checkBorderCollision(ball);
    }
}

// do all collision checks and update the velocity
void Game::checkBallCollision() {
    for (int i = 0; i < MAXBALLS; i++) {
        for (int k = i + 1; k < MAXBALLS; k++) {
            Ball *ball1 = &balls[i];
            Ball *ball2 = &balls[k];

            float distance = hypot(ball1->x - ball2->x, ball1->y - ball2->y);

            if (distance <= ball1->radius + ball2->radius) {
                // ball1 and ball2 are colliding
                // update the velocity of both balls

                while (distance <= ball1->radius + ball2->radius) { // balls go back until they are not overlapping any more
                    ball1->x = ball1->x - ball1->velocity->x * (float) 1 / 100;
                    ball1->y = ball1->y - ball1->velocity->y * (float) 1 / 100;

                    ball2->x = ball2->x - ball2->velocity->x * (float) 1 / 100;
                    ball2->y = ball2->y - ball2->velocity->y * (float) 1 / 100;

                    distance = hypot(ball1->x - ball2->x, ball1->y - ball2->y);
                }

                float m1 = ball1->radius * ball1->radius * PI;
                float m2 = ball2->radius * ball2->radius * PI;

                Vector2D v1 = Vector2D(ball1->velocity->x, ball1->velocity->y);
                Vector2D v2 = Vector2D(ball2->velocity->x, ball2->velocity->y);

                // first ball
                Vector2D tmp1 = v1 - v2;
                Vector2D tmp2 = Vector2D(ball1->x - ball2->x, ball1->y - ball2->y);

                float dot = tmp1.x * tmp2.x + tmp1.y * tmp2.y;
                dot /= (distance * distance);

                float first = (2 * m2 / (m1 + m2));
                float second = dot;
                float third = (ball1->x - ball2->x);

                ball1->velocity->x -= (first * second * third);
                third = (ball1->y - ball2->y);
                ball1->velocity->y -= first * second * third;

                // second ball
                tmp1 = v2 - v1;
                tmp2 = Vector2D(ball2->x - ball1->x, ball2->y - ball1->y);

                dot = tmp1.x * tmp2.x + tmp1.y * tmp2.y;
                dot /= (distance * distance);

                ball2->velocity->x = v2.x - (2 * m1 / (m1 + m2)) * dot * (ball2->x - ball1->x);
                ball2->velocity->y = v2.y - (2 * m1 / (m1 + m2)) * dot * (ball2->y - ball1->y);
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
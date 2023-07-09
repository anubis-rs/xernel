#include "Vector2D.h"

Vector2D::Vector2D() {
    x = 0.0f;
    y = 0.0f;
}

Vector2D::Vector2D(int x, int y) {
    this->x = x;
    this->y = y;
}

Vector2D &Vector2D::Add(const Vector2D &vec) {
    Vector2D *tmp = this;
    tmp->x += vec.x;
    tmp->y += vec.y;
    return *tmp;
}

Vector2D &Vector2D::Sub(const Vector2D &vec) {
    Vector2D *tmp = this;
    tmp->x -= vec.x;
    tmp->y -= vec.y;
    return *tmp;
}

Vector2D &Vector2D::Mul(const Vector2D &vec) {
    Vector2D *tmp = this;
    tmp->x *= vec.x;
    tmp->y *= vec.y;
    return *tmp;
}

Vector2D &Vector2D::Div(const Vector2D &vec) {
    Vector2D *tmp = this;
    tmp->x /= vec.x;
    tmp->y /= vec.y;
    return *tmp;
}

Vector2D & Vector2D::operator+=(const Vector2D &vec) {
    return this->Add(vec);
}

Vector2D & Vector2D::operator-=(const Vector2D &vec) {
    return this->Sub(vec);
}

Vector2D & Vector2D::operator/=(const Vector2D &vec) {
    return this->Div(vec);
}

Vector2D & Vector2D::operator*=(const Vector2D &vec) {
    return this->Mul(vec);
}

Vector2D &Vector2D::Zero() {
    this->x = 0;
    this->y = 0;
    return *this;
}

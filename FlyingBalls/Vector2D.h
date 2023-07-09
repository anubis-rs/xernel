#ifndef SDLTUTORIAL_VECTOR2D_H
#define SDLTUTORIAL_VECTOR2D_H

class Vector2D {
public:
    int x;
    int y;

    Vector2D();

    Vector2D(int x, int y);

    Vector2D &Add(const Vector2D &vec);

    Vector2D &Sub(const Vector2D &vec);

    Vector2D &Mul(const Vector2D &vec);

    Vector2D &Div(const Vector2D &vec);

    Vector2D &operator+=(const Vector2D &vec);

    Vector2D &operator-=(const Vector2D &vec);

    Vector2D &operator*=(const Vector2D &vec);

    Vector2D &operator/=(const Vector2D &vec);

    Vector2D &Zero();
};

#endif //SDLTUTORIAL_VECTOR2D_H

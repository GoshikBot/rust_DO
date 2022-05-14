from optimization import Optimization, Param
from strategy_optimizers import strategy_optimizers

params = (
    Param(value=14.99069, ratio=True, is_int=False),
    Param(value=5.2525, ratio=True, is_int=False),
    Param(value=79, ratio=False, is_int=True),
    Param(value=875.2356, ratio=True, is_int=False),
    Param(value=1.3526, ratio=False, is_int=False),
    Param(value=14, ratio=False, is_int=True),
    Param(value=9.5235362666666, ratio=False, is_int=False),
)

bounds = (
    (1.2, 20),
    (2, 19),
    (5, 80),
    (0.5, 1000),
    (1, 90),
    (10.32, 99),
    (0.1, 10),
)


def function(got_params: tuple) -> tuple:
    result = 0

    for param in got_params:
        if isinstance(param, str):
            param = float(param[:-1])
        value = strategy_optimizers.test_function(param)
        result += value

    return result, got_params


if __name__ == "__main__":
    best_result = Optimization.optimize_all(
        function=function, params=params, bounds=bounds
    )

    print(f"Best result: {best_result}")

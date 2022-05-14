import numpy as np
from scipy.optimize import minimize
from typing import Callable, Union, TypedDict

MIN_DIFF_BETWEEN_ITERATIONS = 30


class DifferentListLengthException(Exception):
    pass


class Param(TypedDict):
    value: float
    ratio: bool
    is_int: bool


class Result(TypedDict):
    value: float
    settings: tuple[Param, ...]


class Optimization:
    results_of_single_param_optimization = []
    method = "Powell"

    @classmethod
    def _gather_params_to_pass(
            cls,
            param: float,
            param_index: int,
            param_ratio: bool,
            is_param_int: bool,
            fixed_params: tuple[Param, ...],
    ) -> tuple[float, ...]:
        result_params = []
        for current_param in fixed_params:
            if current_param["ratio"]:
                result_params.append(f"{current_param['value']}k")
            else:
                result_params.append(
                    int(current_param["value"])
                    if current_param["is_int"]
                    else float(current_param["value"])
                )

        if param_ratio:
            result_params.insert(param_index, f"{param}k")
        else:
            result_params.insert(
                param_index, int(param) if is_param_int else float(param)
            )

        return tuple(result_params)

    @classmethod
    def _get_function_for_optimization(
            cls,
            function: Callable[[tuple], tuple[float, list[Union[float, str]]]],
            fixed_params: tuple[Param, ...],
    ) -> Callable[[tuple[float, ...], int, bool], float]:
        def func_to_optimize(
                param: tuple[float, ...],
                param_index: int,
                param_ratio: bool,
                is_param_int: bool,
        ) -> float:
            params_to_pass = cls._gather_params_to_pass(
                param=param[0],
                param_index=param_index,
                param_ratio=param_ratio,
                is_param_int=is_param_int,
                fixed_params=fixed_params,
            )
            print(f"params_to_pass: {params_to_pass}")

            result, settings = function(params_to_pass)
            settings_params = tuple(
                [
                    Param(
                        value=float(setting[:-1])
                        if isinstance(setting, str)
                        else setting,
                        ratio=isinstance(setting, str),
                        is_int=isinstance(setting, int),
                    )
                    for setting in settings
                ]
            )
            print(f"settings_params: {settings_params}")
            cls.results_of_single_param_optimization.append(
                Result(value=result, settings=settings_params)
            )

            return -result

        return func_to_optimize

    @classmethod
    def optimize_all(
            cls,
            function: Callable[[tuple], tuple[float, list[Union[float, str]]]],
            params: tuple[Param, ...],
            bounds: tuple[tuple[float, float], ...],
            printing: bool = True,
    ) -> Result:
        if len(params) != len(bounds):
            raise DifferentListLengthException(
                "Params and bounds lists have different length"
            )

        cls.results_of_single_param_optimization.clear()

        best_result = Result(value=0, settings=params)
        result_of_all_params_iteration = Result(value=0, settings=params)

        while True:
            for i in range(len(params)):
                print(f"best_result: {best_result}")
                current_param = best_result["settings"][i]

                fixed_params = tuple(
                    [
                        param
                        for param in best_result["settings"]
                        if param is not current_param
                    ]
                )

                best_result = cls.optimize_one(
                    function=function,
                    param=current_param,
                    param_bounds=bounds[i],
                    param_index=i,
                    fixed_params=fixed_params,
                )

                if printing:
                    print(f"best result of optimization of param {i}: {best_result}")

            if (
                    best_result["value"] - result_of_all_params_iteration["value"]
                    < MIN_DIFF_BETWEEN_ITERATIONS
            ):
                return result_of_all_params_iteration

            if printing:
                print(
                    f"current all params iteration best result is greater than previous: {best_result['value']} > {result_of_all_params_iteration['value']}"
                )

            result_of_all_params_iteration = best_result

    @classmethod
    def optimize_one(
            cls,
            function: Callable[[tuple], tuple[float, list[Union[float, str]]]],
            param: Param,
            param_bounds: tuple[float, float],
            param_index: int,
            fixed_params: tuple[Param, ...],
    ) -> Result:
        cls.results_of_single_param_optimization.clear()

        print(f"fixed_params: {fixed_params}")

        tuned_function = cls._get_function_for_optimization(
            function=function, fixed_params=fixed_params
        )
        single_param_array = np.array([param["value"]])
        print(f"single_param_array: {single_param_array}")

        single_param_bounds = (param_bounds,)
        print(f"single_param_bounds: {single_param_bounds}")

        print(f"param_index: {param_index}")
        print(f"param_ratio: {param['ratio']}")

        minimize(
            tuned_function,
            single_param_array,
            args=(param_index, param["ratio"], param["is_int"]),
            bounds=(param_bounds,),
            method=cls.method,
        )

        cls.results_of_single_param_optimization.sort(
            key=lambda r: r["value"], reverse=True
        )
        return cls.results_of_single_param_optimization[0]

from typing import Callable

Null = Callable[[], None]

def profile(function: Null) -> Null:
    def wrapper() -> None:
        # pylint: disable = import-error, import-outside-toplevel
        import yappi  # type: ignore

        yappi.set_clock_type('cpu')
        yappi.start()

        function()

        stats = yappi.get_func_stats()
        # pylint: disable = no-member
        stats.save('yappi.callgrind', type = 'callgrind')

    return wrapper

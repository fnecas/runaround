// error: runaround: auto-wrap: a function must return
#import "/src/lib.typ": *
#show: runaround.with(auto-wrap: (figure: it => 5))
#figure(rect())

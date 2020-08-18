rm -rf graphs/

mkdir graphs/

i=0

python3 generate_graphs.py | while read graph; do
    name=$(printf "%05d" $i)
    echo $name
    echo $graph | dot -Tpng -o graphs/$name.png
    ((i+=1))
done

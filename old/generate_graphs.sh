rm -rf graphs/

mkdir graphs/

i=0

head -n 5 graphs-5.txt | while read graph; do
    name=$(printf "%05d" $i)
    echo $name
    python3 parse_graph.py $graph | dot -Tpng -o graphs/$name.png
    ((i+=1))
done

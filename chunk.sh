clear

curl -X POST http://127.0.0.1:8080/uploads --header "Transfer-Encoding: chunked" --data-binary @- <<EOF 
3
111
3
222
5
33333
0
EOF

echo "--------------------------------------\\n--------------------------------------"

# echo "--------------------------------------"

curl -X POST http://127.0.0.1:8080/uploads --header "Transfer-Encoding: chunked" --data-binary @- <<EOF
5
Hello
2
, 
4
this
D
 is a chunked
18
 transfer encoding test!
0
EOF

echo "--------------------------------------"

curl -X POST http://127.0.0.1:8080/cgi -d "program=test.py&params=[1,2]"
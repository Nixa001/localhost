#!/usr/local/bin/php
<?php
header("Content-Type: text/html");
echo "<html><body>";
echo "<h1>CGI Test</h1>";
echo "<p>SERVER_NAME: " . $_SERVER['SERVER_NAME'] . "</p>";
echo "<p>REQUEST_METHOD: " . $_SERVER['REQUEST_METHOD'] . "</p>";
echo "<p>QUERY_STRING: " . $_SERVER['QUERY_STRING'] . "</p>";
echo "</body></html>";
?>
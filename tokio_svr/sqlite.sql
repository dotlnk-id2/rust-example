CREATE TABLE route_info (
	id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	api_path varchar(255) NOT NULL,
	api_method jsonb NOT NULL,
	api_version varchar(64) NOT NULL,
	api_status INTEGER DEFAULT (0) NOT NULL,
	ctime DATATIME NOT NULL,
	mtime DATATIME NOT NULL
);
